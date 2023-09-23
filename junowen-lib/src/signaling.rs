pub mod client;
pub mod server;
pub mod transfer;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CompressedSessionDesc(pub String);

#[derive(Deserialize, Serialize)]
pub enum SignalingServerMessage {
    RequestOwner,
    RequestAnswer(CompressedSessionDesc),
    SetAnswerDesc(CompressedSessionDesc),
    Delay(u8),
}

#[async_trait]
pub trait SignalingServer {
    async fn send(&mut self, msg: SignalingClientMessage) -> Result<()>;
    async fn recv(&mut self) -> Result<SignalingServerMessage>;
}

#[async_trait]
pub trait SignalingClient {
    async fn send(&mut self, msg: SignalingServerMessage) -> Result<()>;
    async fn recv(&mut self) -> Result<SignalingClientMessage>;
}

#[derive(Deserialize, Serialize)]
pub enum SignalingClientMessage {
    OfferDesc(CompressedSessionDesc),
    AnswerDesc(CompressedSessionDesc),
    Connected,
    Disconnected,
}
