use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;
use junowen_lib::{
    connection::signaling::{
        socket::{OfferResponse, SignalingSocket},
        CompressedSdp,
    },
    signaling_server::{
        reserved_room::{
            PostReservedRoomKeepRequestBody, PostReservedRoomKeepResponse,
            PostReservedRoomKeepResponseOkBody, PutReservedRoomResponse,
        },
        room::{PostRoomJoinRequestBody, PostRoomJoinResponse, PutRoomRequestBody},
    },
};
use tokio::sync::watch;
use tracing::info;

use super::{
    encode_room_name,
    socket::{retry_after, sleep_or_abort_and_delete_room},
};

pub struct SignalingServerReservedRoomOpponentSocket {
    client: reqwest::Client,
    resource_url: String,
    key: Option<String>,
    abort_rx: watch::Receiver<bool>,
}

impl SignalingServerReservedRoomOpponentSocket {
    pub fn new(origin: String, room_name: &str, abort_rx: watch::Receiver<bool>) -> Self {
        let encoded_room_name = encode_room_name(room_name);
        Self {
            client: reqwest::Client::new(),
            resource_url: format!("{}/reserved-room/{}", origin, encoded_room_name),
            key: None,
            abort_rx,
        }
    }

    pub fn into_key(self) -> Option<String> {
        self.key
    }

    async fn sleep_or_abort_and_delete_room(&mut self, retry_after: u32, key: &str) -> Result<()> {
        let url = &self.resource_url;
        sleep_or_abort_and_delete_room(retry_after, &mut self.abort_rx, &self.client, url, key)
            .await
    }
}

#[async_trait]
impl SignalingSocket for SignalingServerReservedRoomOpponentSocket {
    fn timeout() -> Duration {
        Duration::from_secs(10)
    }

    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        let url = &self.resource_url;
        let json = PutRoomRequestBody::new(desc);
        info!("PUT {}", url);
        let res = self.client.put(url).json(&json).send().await?;
        info!("{:?}", res);
        let res =
            PutReservedRoomResponse::parse(res.status(), retry_after(&res), &res.text().await?)?;
        let key = match res {
            PutReservedRoomResponse::Conflict { body, .. } => {
                let Some(offer) = body.into_offer() else {
                    bail!("room is full");
                };
                return Ok(OfferResponse::Offer(offer));
            }
            PutReservedRoomResponse::CreatedWithAnswer { body, .. } => {
                return Ok(OfferResponse::Answer(body.into_answer()));
            }
            PutReservedRoomResponse::CreatedWithKey { retry_after, body } => {
                let key = body.into_key();
                self.sleep_or_abort_and_delete_room(retry_after, &key)
                    .await?;
                key
            }
        };
        self.key = Some(key.clone());

        let url = format!("{}/keep", self.resource_url);
        let body = PostReservedRoomKeepRequestBody::new(key.clone(), None);
        loop {
            info!("POST {}", url);
            let res = self.client.post(&url).json(&body).send().await?;
            info!("{:?}", res);
            let status = res.status();
            let retry_after = retry_after(&res);
            let body = res.text().await.ok();
            let res = PostReservedRoomKeepResponse::parse(status, retry_after, body.as_deref())?;
            info!("{:?}", res);
            match res {
                PostReservedRoomKeepResponse::BadRequest => {
                    bail!("bad request")
                }
                PostReservedRoomKeepResponse::Ok(body) => {
                    let PostReservedRoomKeepResponseOkBody::OpponentAnswer(body) = body else {
                        bail!("invalid response");
                    };
                    return Ok(OfferResponse::Answer(body.into_opponent_answer()));
                }
                PostReservedRoomKeepResponse::NoContent { retry_after } => {
                    self.sleep_or_abort_and_delete_room(retry_after, &key)
                        .await?;
                }
            }
        }
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        let url = format!("{}/join", self.resource_url);
        let json = PostRoomJoinRequestBody::new(desc);
        let res = self.client.post(url).json(&json).send().await?;
        let res = PostRoomJoinResponse::parse(res.status())?;
        match res {
            PostRoomJoinResponse::Ok => Ok(()),
            PostRoomJoinResponse::Conflict => bail!("room is full"),
        }
    }
}
