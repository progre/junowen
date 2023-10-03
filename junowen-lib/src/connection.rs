mod data_channel;
mod peer_connection;
pub mod signaling;

use bytes::Bytes;
use tokio::{spawn, sync::mpsc};
use webrtc::peer_connection::RTCPeerConnection;

use self::data_channel::DataChannel;

pub struct Connection {
    rtc_peer_connection: Option<RTCPeerConnection>,
    data_channel: DataChannel,
    pub message_sender: mpsc::Sender<Bytes>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        let rtc_peer_connection = self.rtc_peer_connection.take().unwrap();
        spawn(async move {
            // NOTE: If the connection was established, it will not be disconnected by drop,
            //       so close it explicitly.
            let _ = rtc_peer_connection.close().await;
        });
    }
}

impl Connection {
    pub fn new(rtc_peer_connection: RTCPeerConnection, data_channel: DataChannel) -> Self {
        let message_sender = data_channel.message_sender.clone();
        Self {
            rtc_peer_connection: Some(rtc_peer_connection),
            data_channel,
            message_sender,
        }
    }

    pub async fn recv(&mut self) -> Option<Bytes> {
        self.data_channel.recv().await
    }
}
