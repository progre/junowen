use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::oneshot;

use super::{
    super::CompressedSdp, async_read_write_socket::SignalingServerMessage, OfferResponse,
    SignalingSocket,
};

pub struct ChannelSocket {
    offer_sender: Option<oneshot::Sender<CompressedSdp>>,
    answer_sender: Option<oneshot::Sender<CompressedSdp>>,
    message_receiver: Option<oneshot::Receiver<SignalingServerMessage>>,
}

impl ChannelSocket {
    pub fn new(
        offer_sender: oneshot::Sender<CompressedSdp>,
        answer_sender: oneshot::Sender<CompressedSdp>,
        message_receiver: oneshot::Receiver<SignalingServerMessage>,
    ) -> Self {
        Self {
            offer_sender: Some(offer_sender),
            answer_sender: Some(answer_sender),
            message_receiver: Some(message_receiver),
        }
    }
}

#[async_trait]
impl SignalingSocket for ChannelSocket {
    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        self.offer_sender.take().unwrap().send(desc).unwrap();
        Ok(match self.message_receiver.take().unwrap().await? {
            SignalingServerMessage::SetAnswerDesc(answer_desc) => {
                OfferResponse::Answer(answer_desc)
            }
            SignalingServerMessage::RequestAnswer(offer_desc) => OfferResponse::Offer(offer_desc),
        })
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        self.answer_sender.take().unwrap().send(desc).unwrap();
        Ok(())
    }
}
