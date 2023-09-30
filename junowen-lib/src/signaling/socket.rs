pub mod async_read_write_socket;

use anyhow::{Context, Result};
use async_trait::async_trait;

use super::{peer_connection::PeerConnection, CompressedSessionDesc};

pub use async_read_write_socket::AsyncReadWriteSocket;

pub enum OfferResponse {
    Offer(CompressedSessionDesc),
    Answer(CompressedSessionDesc),
}

#[async_trait]
pub trait SignalingSocket {
    async fn offer(&mut self, desc: CompressedSessionDesc) -> Result<OfferResponse>;
    async fn answer(&mut self, desc: CompressedSessionDesc) -> Result<()>;

    async fn receive_signaling(&mut self) -> Result<PeerConnection> {
        let mut conn = PeerConnection::new().await?;
        let offer_desc = conn
            .start_as_offerer()
            .await
            .context("Failed to start as host")?;
        let answer_desc = self.offer(offer_desc).await?;
        match answer_desc {
            OfferResponse::Answer(answer_desc) => {
                conn.set_answer_desc(answer_desc)
                    .await
                    .context("Failed to set answer desc")?;
                Ok(conn)
            }
            OfferResponse::Offer(offer_desc) => {
                let mut conn = PeerConnection::new().await?;
                let answer_desc = conn
                    .start_as_answerer(offer_desc)
                    .await
                    .context("Failed to start as guest")?;
                self.answer(answer_desc).await?;
                Ok(conn)
            }
        }
    }
}
