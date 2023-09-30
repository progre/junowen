mod data_channel;
mod peer_connection;
pub mod signaling;

use anyhow::Result;
use bytes::Bytes;
use tokio::sync::{broadcast, mpsc};

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

    pub fn subscribe_disconnected_receiver(&self) -> broadcast::Receiver<()> {
        self.data_channel.pc_disconnected_rx.resubscribe()
    }

    pub async fn close(self) -> Result<()> {
        self.data_channel.close().await
    }
}
