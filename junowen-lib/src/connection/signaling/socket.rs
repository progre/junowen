pub mod async_read_write_socket;
pub mod channel_socket;

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::connection::data_channel::DataChannel;

use super::super::peer_connection::PeerConnection;

use super::CompressedSdp;

pub use async_read_write_socket::AsyncReadWriteSocket;

pub enum OfferResponse {
    Offer(CompressedSdp),
    Answer(CompressedSdp),
}

#[async_trait]
pub trait SignalingSocket {
    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse>;
    async fn answer(&mut self, desc: CompressedSdp) -> Result<()>;

    async fn receive_signaling(&mut self) -> Result<(PeerConnection, DataChannel)> {
        let mut conn = PeerConnection::new().await?;
        let offer_desc = conn
            .start_as_offerer()
            .await
            .context("Failed to start as host")?;
        let answer_desc = self.offer(offer_desc).await?;
        let mut conn = match answer_desc {
            OfferResponse::Answer(answer_desc) => {
                conn.set_answer_desc(answer_desc)
                    .await
                    .context("Failed to set answer desc")?;
                conn
            }
            OfferResponse::Offer(offer_desc) => {
                let mut conn = PeerConnection::new().await?;
                let answer_desc = conn
                    .start_as_answerer(offer_desc)
                    .await
                    .context("Failed to start as guest")?;
                self.answer(answer_desc).await?;
                conn
            }
        };
        let data_channel = conn.wait_for_open_data_channel().await?;
        Ok((conn, data_channel))
    }
}
