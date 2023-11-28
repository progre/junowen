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

use crate::in_game_lobby::waiting_for_match::socket::retry_after;

use super::socket::sleep_or_abort_and_delete_room;

pub struct SignalingServerReservedRoomOpponentSocket {
    client: reqwest::Client,
    origin: String,
    room_name: String,
    key: Option<String>,
    abort_rx: watch::Receiver<bool>,
}

impl SignalingServerReservedRoomOpponentSocket {
    pub fn new(origin: String, room_name: String, abort_rx: watch::Receiver<bool>) -> Self {
        Self {
            client: reqwest::Client::new(),
            origin,
            room_name,
            key: None,
            abort_rx,
        }
    }

    pub fn into_key(self) -> Option<String> {
        self.key
    }

    async fn sleep_or_abort_and_delete_room(&mut self, retry_after: u32, key: &str) -> Result<()> {
        let url = format!("{}/reserved-room/{}", self.origin, self.room_name);
        sleep_or_abort_and_delete_room(retry_after, &mut self.abort_rx, &self.client, &url, key)
            .await
    }
}

#[async_trait]
impl SignalingSocket for SignalingServerReservedRoomOpponentSocket {
    fn timeout() -> Duration {
        Duration::from_secs(10)
    }

    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        let url = format!("{}/reserved-room/{}", self.origin, self.room_name);
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

        let url = format!("{}/reserved-room/{}/keep", self.origin, self.room_name);
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
        let url = format!("{}/reserved-room/{}/join", self.origin, self.room_name);
        let json = PostRoomJoinRequestBody::new(desc);
        let res = self.client.post(url).json(&json).send().await?;
        let res = PostRoomJoinResponse::parse(res.status())?;
        match res {
            PostRoomJoinResponse::Ok => Ok(()),
            PostRoomJoinResponse::Conflict => bail!("room is full"),
        }
    }
}
