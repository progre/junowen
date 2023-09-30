use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::signaling::{SignalingClientMessage, SignalingServer, SignalingServerMessage};

pub struct AsyncReadWriteSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    read_write: T,
}

impl<T> AsyncReadWriteSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    pub fn new(read_write: T) -> Self {
        Self { read_write }
    }

    pub fn into_inner(self) -> T {
        self.read_write
    }
}

#[async_trait]
impl<T> SignalingServer for AsyncReadWriteSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    async fn send(&mut self, msg: SignalingClientMessage) -> anyhow::Result<()> {
        Ok(self
            .read_write
            .write_all(&rmp_serde::to_vec(&msg).unwrap())
            .await?)
    }

    async fn recv(&mut self) -> anyhow::Result<SignalingServerMessage> {
        let mut buf = [0u8; 4 * 1024];
        let len = self.read_write.read(&mut buf).await?;
        Ok(rmp_serde::from_slice(&buf[..len])?)
    }
}
