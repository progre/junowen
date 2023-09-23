use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::signaling::{
    SignalingClient, SignalingClientMessage, SignalingServer, SignalingServerMessage,
};

pub fn create_local_transfer() -> (LocalTransferToServer, LocalTransferToClient) {
    let (to_server_tx, from_client_rx) = mpsc::channel(1);
    let (to_client_tx, from_server_rx) = mpsc::channel(1);
    (
        LocalTransferToServer {
            to_server_tx,
            from_server_rx,
        },
        LocalTransferToClient {
            from_client_rx,
            to_client_tx,
        },
    )
}

pub struct LocalTransferToClient {
    from_client_rx: mpsc::Receiver<SignalingClientMessage>,
    to_client_tx: mpsc::Sender<SignalingServerMessage>,
}

#[async_trait]
impl SignalingClient for LocalTransferToClient {
    async fn send(&mut self, msg: SignalingServerMessage) -> anyhow::Result<()> {
        Ok(self.to_client_tx.send(msg).await?)
    }

    async fn recv(&mut self) -> anyhow::Result<SignalingClientMessage> {
        self.from_client_rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("channel closed"))
    }
}

pub struct LocalTransferToServer {
    to_server_tx: mpsc::Sender<SignalingClientMessage>,
    from_server_rx: mpsc::Receiver<SignalingServerMessage>,
}

#[async_trait]
impl SignalingServer for LocalTransferToServer {
    async fn send(&mut self, msg: SignalingClientMessage) -> anyhow::Result<()> {
        Ok(self.to_server_tx.send(msg).await?)
    }

    async fn recv(&mut self) -> anyhow::Result<SignalingServerMessage> {
        self.from_server_rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("channel closed"))
    }
}
