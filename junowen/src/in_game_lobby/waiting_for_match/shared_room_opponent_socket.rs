use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;

use junowen_lib::{
    connection::signaling::{
        socket::{OfferResponse, SignalingSocket},
        CompressedSdp,
    },
    signaling_server::{
        custom::{
            PostSharedRoomKeepRequestBody, PostSharedRoomKeepResponse, PutSharedRoomResponse,
        },
        room::{PostRoomJoinRequestBody, PostRoomJoinResponse, PutRoomRequestBody},
    },
};
use tokio::sync::watch;
use tracing::info;

use crate::in_game_lobby::waiting_for_match::socket::retry_after;

use super::socket::sleep_or_abort_and_delete_room;

pub struct SignalingServerSharedRoomOpponentSocket {
    client: reqwest::Client,
    origin: String,
    room_name: String,
    abort_rx: watch::Receiver<bool>,
}

impl SignalingServerSharedRoomOpponentSocket {
    pub fn new(origin: String, room_name: String, abort_rx: watch::Receiver<bool>) -> Self {
        Self {
            client: reqwest::Client::new(),
            origin,
            room_name,
            abort_rx,
        }
    }

    async fn sleep_or_abort_and_delete_room(&mut self, retry_after: u32, key: &str) -> Result<()> {
        let url = format!("{}/custom/{}", self.origin, self.room_name);
        sleep_or_abort_and_delete_room(retry_after, &mut self.abort_rx, &self.client, &url, key)
            .await
    }
}

#[async_trait]
impl SignalingSocket for SignalingServerSharedRoomOpponentSocket {
    fn timeout() -> Duration {
        Duration::from_secs(10)
    }

    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        let url = format!("{}/custom/{}", self.origin, self.room_name);
        let json = PutRoomRequestBody::new(desc);
        info!("PUT {}", url);
        let res = self.client.put(url).json(&json).send().await?;
        let res =
            PutSharedRoomResponse::parse(res.status(), retry_after(&res), &res.text().await?)?;
        info!("{:?}", res);
        let key = match res {
            PutSharedRoomResponse::Conflict { body, .. } => {
                return Ok(OfferResponse::Offer(body.into_offer()))
            }
            PutSharedRoomResponse::CreatedWithAnswer { body, .. } => {
                return Ok(OfferResponse::Answer(body.into_answer()));
            }
            PutSharedRoomResponse::CreatedWithKey { retry_after, body } => {
                let key = body.into_key();
                self.sleep_or_abort_and_delete_room(retry_after, &key)
                    .await?;
                key
            }
        };

        let url = format!("{}/custom/{}/keep", self.origin, self.room_name);
        let body = PostSharedRoomKeepRequestBody::new(key.clone());
        loop {
            info!("POST {}", url);
            let res = self.client.post(&url).json(&body).send().await?;
            let status = res.status();
            let retry_after = retry_after(&res);
            let body = res.text().await.ok();
            let res = PostSharedRoomKeepResponse::parse(status, retry_after, body.as_deref())?;
            info!("{:?}", res);
            match res {
                PostSharedRoomKeepResponse::BadRequest => {
                    bail!("bad request")
                }
                PostSharedRoomKeepResponse::Ok(body) => {
                    return Ok(OfferResponse::Answer(body.into_answer()));
                }
                PostSharedRoomKeepResponse::NoContent { retry_after } => {
                    self.sleep_or_abort_and_delete_room(retry_after, &key)
                        .await?;
                }
            }
        }
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        let url = format!("{}/custom/{}/join", self.origin, self.room_name);
        let json = PostRoomJoinRequestBody::new(desc);
        let res = self.client.post(url).json(&json).send().await?;
        let res = PostRoomJoinResponse::parse(res.status())?;
        match res {
            PostRoomJoinResponse::Ok => Ok(()),
            PostRoomJoinResponse::Conflict => bail!("room is full"),
        }
    }
}
