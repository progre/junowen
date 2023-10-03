use std::io::Write;

use anyhow::{anyhow, bail, Result};
use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine};
use flate2::{
    write::{DeflateDecoder, DeflateEncoder},
    Compression,
};
use regex::Regex;
use tokio::{
    select,
    sync::{broadcast, oneshot},
};
use tracing::debug;
use webrtc::{
    api::media_engine::MediaEngine,
    data_channel::data_channel_init::RTCDataChannelInit,
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::{sdp_type::RTCSdpType, session_description::RTCSessionDescription},
        RTCPeerConnection,
    },
};

use super::{data_channel::DataChannel, signaling::CompressedSessionDesc};

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

pub struct PeerConnection {
    rtc: RTCPeerConnection,
    peer_connection_state_disconnected_rx: Option<broadcast::Receiver<()>>,
    peer_connection_state_failed_rx: Option<oneshot::Receiver<()>>,
    data_channel_rx: Option<oneshot::Receiver<DataChannel>>,
}

unsafe impl Send for PeerConnection {}
unsafe impl Sync for PeerConnection {}

const PROTOCOL: &str = "JUNOWEN/0.0";

impl PeerConnection {
    pub async fn new() -> Result<Self> {
        let rtc = create_default_peer_connection().await?;

        let (peer_connection_state_failed_tx, peer_connection_state_failed_rx) = oneshot::channel();
        let mut peer_connection_state_failed_tx = Some(peer_connection_state_failed_tx);

        let (peer_connection_state_disconnected_tx, peer_connection_state_disconnected_rx) =
            broadcast::channel(1);
        let mut peer_connection_state_disconnected_tx = Some(peer_connection_state_disconnected_tx);

        // All events (useful for debugging)
        // rtc.on_ice_candidate(Box::new(|_candidate| Box::pin(async {})));
        // rtc.on_ice_connection_state_change(Box::new(|_state| Box::pin(async {})));
        // rtc.on_ice_gathering_state_change(Box::new(|_state| Box::pin(async {})));
        // rtc.on_negotiation_needed(Box::new(|| Box::pin(async {})));
        rtc.on_peer_connection_state_change(Box::new(move |state| {
            // NOTE: RTCDataChannel cannot detect the disconnection
            //       of RTCPeerConnection, so it is transmitted by channel.
            debug!("on_peer_connection_state_change {}", state);
            match state {
                RTCPeerConnectionState::Failed => {
                    let tx = peer_connection_state_failed_tx.take().unwrap();
                    let _ = tx.send(());
                    Box::pin(async move {})
                }
                RTCPeerConnectionState::Disconnected => {
                    let tx = peer_connection_state_disconnected_tx.take().unwrap();
                    let _ = tx.send(());
                    Box::pin(async move {})
                }
                _ => Box::pin(async move {}),
            }
        }));
        // rtc.on_signaling_state_change(Box::new(|_state| Box::pin(async {})));
        // rtc.on_track(Box::new(|_track, _receiver, _transceiver| {
        //     Box::pin(async {})
        // }));

        Ok(Self {
            rtc,
            peer_connection_state_failed_rx: Some(peer_connection_state_failed_rx),
            peer_connection_state_disconnected_rx: Some(peer_connection_state_disconnected_rx),
            data_channel_rx: None,
        })
    }

    pub async fn start_as_offerer(&mut self) -> Result<CompressedSessionDesc> {
        let rtc_data_channel = self
            .rtc
            .create_data_channel(
                "data",
                Some(RTCDataChannelInit {
                    protocol: Some(PROTOCOL.to_owned()),
                    ..Default::default()
                }),
            )
            .await?;
        let disconnected_rx = self.peer_connection_state_disconnected_rx.take().unwrap();
        let (data_channel_tx, data_channel_rx) = oneshot::channel();
        self.data_channel_rx = Some(data_channel_rx);
        let _ = data_channel_tx.send(DataChannel::new(rtc_data_channel, disconnected_rx).await);

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

    pub async fn start_as_answerer(
        &mut self,
        offer_desc: CompressedSessionDesc,
    ) -> Result<CompressedSessionDesc> {
        let (data_channel_tx, data_channel_rx) = oneshot::channel();
        self.data_channel_rx = Some(data_channel_rx);
        let mut data_channel_tx = Some(data_channel_tx);
        let mut disconnected_rx = Some(self.peer_connection_state_disconnected_rx.take().unwrap());
        self.rtc.on_data_channel(Box::new(move |rtc_data_channel| {
            let data_channel_tx = data_channel_tx.take().unwrap();
            let disconnected_rx = disconnected_rx.take().unwrap();
            Box::pin(async move {
                let _ =
                    data_channel_tx.send(DataChannel::new(rtc_data_channel, disconnected_rx).await);
            })
        }));
        let offer_desc = decompress(&offer_desc.0)?;
        self.rtc.set_remote_description(offer_desc).await?;
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

    pub async fn set_answer_desc(&self, answer_desc: CompressedSessionDesc) -> Result<()> {
        self.rtc
            .set_remote_description(decompress(&answer_desc.0)?)
            .await?;
        Ok(())
    }

    pub async fn wait_for_open_data_channel(mut self) -> Result<(RTCPeerConnection, DataChannel)> {
        let data_channel_task = async {
            let mut data_channel = self.data_channel_rx.take().unwrap().await.unwrap();
            data_channel.wait_for_open_data_channel().await;
            if data_channel.protocol() != PROTOCOL {
                // TODO: ここでエラーを返すとコネクションがリークするかも
                bail!("unexpected protocol: {}", data_channel.protocol());
            }
            Ok(data_channel)
        };
        let failed_task = self.peer_connection_state_failed_rx.take().unwrap();
        select! {
            result = data_channel_task => result.map(|data_channel| (self.rtc, data_channel)),
            _ = failed_task => bail!("RTCPeerConnection failed"),
        }
    }
}
