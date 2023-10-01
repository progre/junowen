mod data_channel;
mod peer_connection;
pub mod signaling;

use bytes::Bytes;
use tokio::sync::mpsc;

use self::data_channel::DataChannel;

pub struct Connection {
    data_channel: DataChannel,
    pub message_sender: mpsc::Sender<Bytes>,
}

impl Connection {
    pub fn new(data_channel: DataChannel) -> Self {
        let message_sender = data_channel.message_sender.clone();
        Self {
            data_channel,
            message_sender,
        }
    }

    pub async fn recv(&mut self) -> Option<Bytes> {
        self.data_channel.recv().await
    }
}
