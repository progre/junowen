mod peer_connection;
mod stdio_mock_connection;

use anyhow::{Context, Result};
use async_trait::async_trait;

pub use self::{
    peer_connection::DataChannel, peer_connection::PeerConnectionImpl,
    stdio_mock_connection::StdioMockConnection,
};

use super::{
    CompressedSessionDesc, SignalingClientMessage, SignalingServer, SignalingServerMessage,
};

#[async_trait]
pub trait PeerConnection {
    async fn start_as_host(&mut self) -> Result<CompressedSessionDesc>;
    async fn start_as_guest(
        &mut self,
        offer_desc: CompressedSessionDesc,
    ) -> Result<CompressedSessionDesc>;
    async fn set_answer_desc(&self, answer_desc: CompressedSessionDesc) -> Result<()>;
}

pub async fn receive_signaling(
    server: &mut impl SignalingServer,
    conn: &mut impl PeerConnection,
) -> Result<()> {
    let msg = server.recv().await?;
    match msg {
        SignalingServerMessage::RequestOwner => {
            let offer_desc = conn
                .start_as_host()
                .await
                .context("Failed to start as host")?;
            server
                .send(SignalingClientMessage::OfferDesc(offer_desc))
                .await?;
            let SignalingServerMessage::SetAnswerDesc(answer_desc) = server.recv().await? else {
                panic!("unexpected message");
            };
            conn.set_answer_desc(answer_desc)
                .await
                .context("Failed to set answer desc")?;
            Ok(())
        }
        SignalingServerMessage::RequestAnswer(offer_desc) => {
            let answer_desc = conn
                .start_as_guest(offer_desc)
                .await
                .context("Failed to start as guest")?;
            server
                .send(SignalingClientMessage::AnswerDesc(answer_desc))
                .await?;
            Ok(())
        }
        _ => panic!("unexpected message"),
    }
}
