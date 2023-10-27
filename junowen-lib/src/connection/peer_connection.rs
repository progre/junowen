use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::{
    select,
    sync::{broadcast, oneshot},
};
use tracing::{debug, trace};
use webrtc::{
    api::setting_engine::SettingEngine,
    data_channel::data_channel_init::RTCDataChannelInit,
    ice_transport::ice_server::RTCIceServer,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::sdp_type::RTCSdpType, RTCPeerConnection,
    },
};

use super::{
    data_channel::DataChannel,
    signaling::{decompress_session_description, CompressedSdp},
};

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
    let mut setting_engine = SettingEngine::default();
    setting_engine.set_ice_timeouts(None, Some(Duration::from_secs(20 * 60)), None);
    Ok(webrtc::api::APIBuilder::new()
        .with_setting_engine(setting_engine)
        .build()
        .new_peer_connection(create_default_config())
        .await?)
}

pub struct PeerConnection {
    rtc: Option<RTCPeerConnection>,
    peer_connection_state_disconnected_rx: Option<broadcast::Receiver<()>>,
    peer_connection_state_failed_rx: Option<oneshot::Receiver<()>>,
    data_channel_rx: Option<oneshot::Receiver<DataChannel>>,
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        trace!("drop connection");
        let rtc_peer_connection = self.rtc.take().unwrap();
        let drop = async move {
            // NOTE: If the connection was established, it will not be disconnected by drop,
            //       so close it explicitly.
            let _ = rtc_peer_connection.close().await;
            trace!("connection closed");
        };
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.spawn(drop);
        } else {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(drop);
        }
    }
}

const PROTOCOL: &str = "JUNOWEN/0.5";

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
            rtc: Some(rtc),
            peer_connection_state_failed_rx: Some(peer_connection_state_failed_rx),
            peer_connection_state_disconnected_rx: Some(peer_connection_state_disconnected_rx),
            data_channel_rx: None,
        })
    }

    fn rtc(&self) -> &RTCPeerConnection {
        self.rtc.as_ref().unwrap()
    }

    pub async fn start_as_offerer(&mut self) -> Result<CompressedSdp> {
        let rtc_data_channel = self
            .rtc()
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

        let offer = self.rtc().create_offer(None).await?;

        let mut gather_complete = self.rtc().gathering_complete_promise().await;
        self.rtc().set_local_description(offer).await?;
        let _ = gather_complete.recv().await;

        let local_desc = self
            .rtc()
            .local_description()
            .await
            .ok_or_else(|| anyhow!("Failed to get local description"))?;
        Ok(CompressedSdp::compress(&local_desc))
    }

    pub async fn start_as_answerer(&mut self, offer_desc: CompressedSdp) -> Result<CompressedSdp> {
        let (data_channel_tx, data_channel_rx) = oneshot::channel();
        self.data_channel_rx = Some(data_channel_rx);
        let mut data_channel_tx = Some(data_channel_tx);
        let mut disconnected_rx = Some(self.peer_connection_state_disconnected_rx.take().unwrap());
        self.rtc()
            .on_data_channel(Box::new(move |rtc_data_channel| {
                let data_channel_tx = data_channel_tx.take().unwrap();
                let disconnected_rx = disconnected_rx.take().unwrap();
                Box::pin(async move {
                    let _ = data_channel_tx
                        .send(DataChannel::new(rtc_data_channel, disconnected_rx).await);
                })
            }));
        let offer_desc = decompress_session_description(RTCSdpType::Offer, offer_desc)?;
        self.rtc().set_remote_description(offer_desc).await?;
        let offer = self.rtc().create_answer(None).await?;

        let mut gather_complete = self.rtc().gathering_complete_promise().await;
        self.rtc().set_local_description(offer).await?;
        let _ = gather_complete.recv().await;

        let local_desc = self
            .rtc()
            .local_description()
            .await
            .ok_or_else(|| anyhow!("Failed to get local description"))?;

        Ok(CompressedSdp::compress(&local_desc))
    }

    pub async fn set_answer_desc(&self, answer_desc: CompressedSdp) -> Result<()> {
        let answer_desc = decompress_session_description(RTCSdpType::Answer, answer_desc)?;
        self.rtc().set_remote_description(answer_desc).await?;
        Ok(())
    }

    pub async fn wait_for_open_data_channel(&mut self) -> Result<DataChannel> {
        let data_channel_task = async {
            let mut data_channel = self.data_channel_rx.take().unwrap().await.unwrap();
            data_channel.wait_for_open_data_channel().await;
            if data_channel.protocol() != PROTOCOL {
                bail!("unexpected protocol: {}", data_channel.protocol());
            }
            Ok(data_channel)
        };
        let failed_task = self.peer_connection_state_failed_rx.take().unwrap();
        select! {
            result = data_channel_task => result,
            _ = failed_task => bail!("RTCPeerConnection failed"),
        }
    }
}
