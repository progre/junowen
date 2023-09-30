use std::io;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::{super::CompressedSessionDesc, OfferResponse, SignalingSocket};

#[derive(Deserialize, Serialize)]
pub enum SignalingServerMessage {
    RequestOwner,
    RequestAnswer(CompressedSessionDesc),
    SetAnswerDesc(CompressedSessionDesc),
    Delay(u8),
}

#[derive(Deserialize, Serialize)]
pub enum SignalingClientMessage {
    OfferDesc(CompressedSessionDesc),
    AnswerDesc(CompressedSessionDesc),
    Connected,
    Disconnected,
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

    async fn send(&mut self, msg: SignalingClientMessage) -> anyhow::Result<(), io::Error> {
        self.read_write
            .write_all(&rmp_serde::to_vec(&msg).unwrap())
            .await
    }

    async fn recv(&mut self) -> anyhow::Result<SignalingServerMessage, io::Error> {
        let mut buf = [0u8; 4 * 1024];
        let len = self.read_write.read(&mut buf).await?;
        Ok(rmp_serde::from_slice(&buf[..len]).unwrap())
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
    async fn offer(&mut self, desc: CompressedSessionDesc) -> Result<OfferResponse> {
        self.send(SignalingClientMessage::OfferDesc(desc)).await?;
        Ok(match self.recv().await? {
            SignalingServerMessage::SetAnswerDesc(answer_desc) => {
                OfferResponse::Answer(answer_desc)
            }
            SignalingServerMessage::RequestAnswer(offer_desc) => OfferResponse::Offer(offer_desc),
            _ => panic!("unexpected message"),
        })
    }

    async fn answer(&mut self, desc: CompressedSessionDesc) -> Result<()> {
        Ok(self.send(SignalingClientMessage::AnswerDesc(desc)).await?)
    }
}
