use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::signaling::{
    SignalingClient, SignalingClientMessage, SignalingServer, SignalingServerMessage,
};

pub struct AsyncReadWriteTransfer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    pipe: T,
}

impl<T> AsyncReadWriteTransfer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    pub fn new(pipe: T) -> Self {
        Self { pipe }
    }

    pub fn into_inner(self) -> T {
        self.pipe
    }
}

#[async_trait]
impl<T> SignalingClient for AsyncReadWriteTransfer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    async fn send(&mut self, msg: SignalingServerMessage) -> anyhow::Result<()> {
        Ok(self
            .pipe
            .write_all(&rmp_serde::to_vec(&msg).unwrap())
            .await?)
    }

    async fn recv(&mut self) -> anyhow::Result<SignalingClientMessage> {
        let mut buf = [0u8; 4 * 1024];
        let len = self.pipe.read(&mut buf).await?;
        Ok(rmp_serde::from_slice(&buf[..len])?)
    }
}

#[async_trait]
impl<T> SignalingServer for AsyncReadWriteTransfer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    async fn send(&mut self, msg: SignalingClientMessage) -> anyhow::Result<()> {
        Ok(self
            .pipe
            .write_all(&rmp_serde::to_vec(&msg).unwrap())
            .await?)
    }

    async fn recv(&mut self) -> anyhow::Result<SignalingServerMessage> {
        let mut buf = [0u8; 4 * 1024];
        let len = self.pipe.read(&mut buf).await?;
        Ok(rmp_serde::from_slice(&buf[..len])?)
    }
}
