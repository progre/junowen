use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use super::{
    super::CompressedSdp,
    OfferResponse, SignalingSocket,
    async_read_write_socket::{SignalingClientMessage, SignalingServerMessage},
    frame::{read_frame, write_frame},
};

/// [`super::AsyncReadWriteSocket`] の対向。常に自身が offerer となる。
pub struct AsyncReadWriteServerSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    read_write: T,
}

impl<T> AsyncReadWriteServerSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    pub fn new(read_write: T) -> Self {
        Self { read_write }
    }
}

#[async_trait]
impl<T> SignalingSocket for AsyncReadWriteServerSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    fn timeout() -> Duration {
        Duration::from_secs(20 * 60)
    }

    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        // 双方が offer を生成済みなので、クライアントの offer は捨てさせる
        let SignalingClientMessage::OfferDesc(_) = read_frame(&mut self.read_write).await? else {
            bail!("unexpected message");
        };
        write_frame(
            &mut self.read_write,
            SignalingServerMessage::RequestAnswer(desc),
        )
        .await?;
        let SignalingClientMessage::AnswerDesc(answer_desc) =
            read_frame(&mut self.read_write).await?
        else {
            bail!("unexpected message");
        };
        Ok(OfferResponse::Answer(answer_desc))
    }

    async fn answer(&mut self, _desc: CompressedSdp) -> Result<()> {
        unreachable!()
    }
}
