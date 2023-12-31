use std::{io, time::Duration};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::{super::CompressedSdp, OfferResponse, SignalingSocket};

#[derive(Debug, Deserialize, Serialize)]
pub enum SignalingServerMessage {
    RequestAnswer(CompressedSdp),
    SetAnswerDesc(CompressedSdp),
}

#[derive(Deserialize, Serialize)]
pub enum SignalingClientMessage {
    OfferDesc(CompressedSdp),
    AnswerDesc(CompressedSdp),
}

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

    async fn send(&mut self, msg: SignalingClientMessage) -> Result<(), io::Error> {
        self.read_write
            .write_all(&rmp_serde::to_vec(&msg).unwrap())
            .await
    }

    async fn recv(&mut self) -> Result<SignalingServerMessage> {
        let mut buf = [0u8; 4 * 1024];
        let len = self.read_write.read(&mut buf).await?;
        rmp_serde::from_slice(&buf[..len])
            .map_err(|err| anyhow!("parse failed (len={}): {}", len, err))
    }

    pub fn into_inner(self) -> T {
        self.read_write
    }
}

#[async_trait]
impl<T> SignalingSocket for AsyncReadWriteSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    fn timeout() -> Duration {
        Duration::from_secs(20 * 60)
    }

    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        self.send(SignalingClientMessage::OfferDesc(desc)).await?;
        Ok(match self.recv().await? {
            SignalingServerMessage::SetAnswerDesc(answer_desc) => {
                OfferResponse::Answer(answer_desc)
            }
            SignalingServerMessage::RequestAnswer(offer_desc) => OfferResponse::Offer(offer_desc),
        })
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        Ok(self.send(SignalingClientMessage::AnswerDesc(desc)).await?)
    }
}
