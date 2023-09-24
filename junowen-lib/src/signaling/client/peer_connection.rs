use std::{io::Write, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine};
use bytes::Bytes;
use flate2::{
    write::{DeflateDecoder, DeflateEncoder},
    Compression,
};
use regex::Regex;
use tokio::{
    spawn,
    sync::{mpsc, watch},
    time::sleep,
};
use webrtc::{
    api::media_engine::MediaEngine,
    data_channel::{data_channel_init::RTCDataChannelInit, RTCDataChannel},
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::{sdp_type::RTCSdpType, session_description::RTCSessionDescription},
        RTCPeerConnection,
    },
};

use crate::signaling::CompressedSessionDesc;

use super::PeerConnection;

fn compress(desc: &RTCSessionDescription) -> String {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
    e.write_all(desc.sdp.as_bytes()).unwrap();
    let compressed_bytes = e.finish().unwrap();

    format!(
        r#"<{}>{}</{}>"#,
        desc.sdp_type,
        BASE64_STANDARD_NO_PAD.encode(compressed_bytes),
        desc.sdp_type,
    )
}

fn decompress(desc: &str) -> Result<RTCSessionDescription> {
    let captures = Regex::new(r#"<(.+?)>(.+?)</(.+?)>"#)
        .unwrap()
        .captures(desc)
        .ok_or_else(|| anyhow!("Failed to parse"))?;
    let sdp_type = &captures[1];
    let sdp_type_end = &captures[3];
    if sdp_type != sdp_type_end {
        bail!("unmatched tag: <{}></{}>", sdp_type, sdp_type_end);
    }
    let compressed_bytes = BASE64_STANDARD_NO_PAD.decode(captures[2].replace(['\n', ' '], ""))?;

    let mut d = DeflateDecoder::new(Vec::new());
    d.write_all(&compressed_bytes)?;
    let sdp = String::from_utf8_lossy(&d.finish()?).to_string();
    Ok(match RTCSdpType::from(sdp_type) {
        RTCSdpType::Offer => RTCSessionDescription::offer(sdp),
        RTCSdpType::Pranswer => RTCSessionDescription::pranswer(sdp),
        RTCSdpType::Answer => RTCSessionDescription::answer(sdp),
        RTCSdpType::Unspecified | RTCSdpType::Rollback => {
            bail!("Failed to parse from {:?}", desc)
        }
    }?)
}

fn create_default_config() -> RTCConfiguration {
    RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    }
}

async fn create_default_peer_connection() -> Result<RTCPeerConnection> {
    Ok(webrtc::api::APIBuilder::new()
        .with_interceptor_registry(Registry::new())
        .with_media_engine(MediaEngine::default())
        .build()
        .new_peer_connection(create_default_config())
        .await?)
}

pub struct DataChannel {
    rtc: Arc<RTCDataChannel>,
    open_rx: mpsc::Receiver<()>,
    pub message_sender: mpsc::Sender<Bytes>,
    incoming_message_rx: mpsc::Receiver<Bytes>,
    close_rx: mpsc::Receiver<()>,
    pc_disconnected_rx: watch::Receiver<()>,
}

impl DataChannel {
    pub async fn new(
        rtc: Arc<RTCDataChannel>,
        pc_disconnected_receiver: watch::Receiver<()>,
    ) -> Self {
        let (open_tx, open_rx) = mpsc::channel(1);
        let (message_sender, mut outgoing_message_receiver) = mpsc::channel(1);
        let (incoming_message_tx, incoming_message_rx) = mpsc::channel(1);
        let (close_sender, close_receiver) = mpsc::channel(1);
        rtc.on_open(Box::new(move || {
            let open_sender = open_tx.clone();
            Box::pin(async move { open_sender.send(()).await.unwrap() })
        }));
        rtc.on_message(Box::new(move |msg| {
            let incoming_message_tx = incoming_message_tx.clone();
            Box::pin(async move { incoming_message_tx.send(msg.data).await.unwrap() })
        }));
        rtc.on_error(Box::new(|err| {
            eprintln!("{}", err);
            Box::pin(async {})
        }));
        rtc.on_close(Box::new(move || {
            let close_sender = close_sender.clone();
            Box::pin(async move {
                let _ = close_sender.send(()).await;
            })
        }));
        rtc.on_buffered_amount_low(Box::new(|| Box::pin(async {})))
            .await;

        {
            // NOTE: To make it possible to have separate references for receiving and sending,
            //       sending is implemented with a channel and a task.
            //       Or, it would be nice to have something like tokio::io::{ReadHalf, WriteHalf}.
            let rtc = rtc.clone();
            spawn(async move {
                while let Some(data) = outgoing_message_receiver.recv().await {
                    let result = rtc.send(&data).await;
                    if let Err(webrtc::Error::ErrClosedPipe) = result {
                        return;
                    } else if let Err(err) = result {
                        eprintln!("{}", err);
                    }
                }
            });
        }

        Self {
            rtc,
            open_rx,
            message_sender,
            incoming_message_rx,
            close_rx: close_receiver,
            pc_disconnected_rx: pc_disconnected_receiver,
        }
    }

    /// This method returns `None` if either `incoming_message_rx`,
    /// `RTCDataChannel`, or `RTCPeerConnection` is closed.
    pub async fn recv(&mut self) -> Option<Bytes> {
        tokio::select! {
            result = self.incoming_message_rx.recv() => result,
            _ = self.close_rx.recv() => None,
            _ = self.pc_disconnected_rx.changed() => None,
        }
    }

    pub async fn close(&self) -> Result<()> {
        Ok(self.rtc.close().await?)
    }
}

pub struct PeerConnectionImpl {
    rtc: RTCPeerConnection,
    disconnected_rx: watch::Receiver<()>,
    data_channel_tx: mpsc::Sender<DataChannel>,
    data_channel_rx: mpsc::Receiver<DataChannel>,
}

unsafe impl Send for PeerConnectionImpl {}
unsafe impl Sync for PeerConnectionImpl {}

impl PeerConnectionImpl {
    pub async fn new() -> Result<Self> {
        let rtc = create_default_peer_connection().await?;

        let (data_channel_tx, data_channel_rx) = mpsc::channel(1);
        let (disconnected_tx, disconnected_rx) = watch::channel(());

        // All events (useful for debugging)
        {
            let data_channel_tx = data_channel_tx.clone();
            let disconnected_rx = disconnected_rx.clone();
            rtc.on_data_channel(Box::new(move |rtc_data_channel| {
                let data_channel_tx = data_channel_tx.clone();
                let disconnected_rx = disconnected_rx.clone();
                Box::pin(async move {
                    Self::send_data_channel(&data_channel_tx, rtc_data_channel, disconnected_rx)
                        .await
                        .unwrap();
                })
            }));
        }
        // rtc.on_ice_candidate(Box::new(|_candidate| Box::pin(async {})));
        // rtc.on_ice_connection_state_change(Box::new(|_state| Box::pin(async {})));
        // rtc.on_ice_gathering_state_change(Box::new(|_state| Box::pin(async {})));
        // rtc.on_negotiation_needed(Box::new(|| Box::pin(async {})));
        rtc.on_peer_connection_state_change(Box::new(move |state| {
            // NOTE: RTCDataChannel cannot detect the disconnection
            //       of RTCPeerConnection, so it is transmitted by channel.
            if let RTCPeerConnectionState::Disconnected = state {
                disconnected_tx.send(()).unwrap();
            }
            Box::pin(async {})
        }));
        // rtc.on_signaling_state_change(Box::new(|_state| Box::pin(async {})));
        // rtc.on_track(Box::new(|_track, _receiver, _transceiver| {
        //     Box::pin(async {})
        // }));

        Ok(Self {
            rtc,
            disconnected_rx,
            data_channel_tx,
            data_channel_rx,
        })
    }

    async fn send_data_channel(
        data_channel_sender: &mpsc::Sender<DataChannel>,
        rtc_data_channel: Arc<RTCDataChannel>,
        disconnected_receiver: watch::Receiver<()>,
    ) -> Result<()> {
        let data_channel = DataChannel::new(rtc_data_channel, disconnected_receiver).await;
        data_channel_sender.send(data_channel).await?;
        Ok(())
    }

    pub async fn wait_for_open_data_channel(&mut self, duration: Duration) -> Option<DataChannel> {
        let task = async {
            let mut data_channel = self.data_channel_rx.recv().await.unwrap();
            data_channel.open_rx.recv().await.unwrap();
            Some(data_channel)
        };
        tokio::select! {
            _ = sleep(duration) => None,
            some = task => some,
        }
    }
}

#[async_trait]
impl PeerConnection for PeerConnectionImpl {
    async fn start_as_host(&mut self) -> Result<CompressedSessionDesc> {
        let rtc_data_channel = self
            .rtc
            .create_data_channel(
                "data",
                Some(RTCDataChannelInit {
                    protocol: Some("JUNOWEN/1.0".to_owned()),
                    ..Default::default()
                }),
            )
            .await?;
        Self::send_data_channel(
            &self.data_channel_tx,
            rtc_data_channel,
            self.disconnected_rx.clone(),
        )
        .await?;

        let offer = self.rtc.create_offer(None).await?;

        let mut gather_complete = self.rtc.gathering_complete_promise().await;
        self.rtc.set_local_description(offer).await?;
        let _ = gather_complete.recv().await;

        let local_desc = self
            .rtc
            .local_description()
            .await
            .ok_or_else(|| anyhow!("Failed to get local description"))?;
        Ok(CompressedSessionDesc(compress(&local_desc)))
    }

    async fn start_as_guest(
        &mut self,
        offer_desc: CompressedSessionDesc,
    ) -> Result<CompressedSessionDesc> {
        self.rtc
            .set_remote_description(decompress(&offer_desc.0)?)
            .await?;
        let offer = self.rtc.create_answer(None).await?;

        let mut gather_complete = self.rtc.gathering_complete_promise().await;
        self.rtc.set_local_description(offer).await?;
        let _ = gather_complete.recv().await;

        let local_desc = self
            .rtc
            .local_description()
            .await
            .ok_or_else(|| anyhow!("Failed to get local description"))?;

        Ok(CompressedSessionDesc(compress(&local_desc)))
    }

    async fn set_answer_desc(&self, answer_desc: CompressedSessionDesc) -> Result<()> {
        self.rtc
            .set_remote_description(decompress(&answer_desc.0)?)
            .await?;
        Ok(())
    }
}
