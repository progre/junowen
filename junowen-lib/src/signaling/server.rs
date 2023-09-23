use anyhow::Result;

use super::{SignalingClient, SignalingClientMessage, SignalingServerMessage};

pub async fn serve_signaling(
    host: &mut impl SignalingClient,
    guest: &mut impl SignalingClient,
) -> Result<()> {
    host.send(SignalingServerMessage::RequestOwner).await?;
    let SignalingClientMessage::OfferDesc(offer_desc) = host.recv().await? else {
        panic!("unexpected message");
    };
    guest
        .send(SignalingServerMessage::RequestAnswer(offer_desc))
        .await?;
    let SignalingClientMessage::AnswerDesc(answer_desc) = guest.recv().await? else {
        panic!("unexpected message");
    };
    host.send(SignalingServerMessage::SetAnswerDesc(answer_desc))
        .await?;
    Ok(())
}
