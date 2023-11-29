use std::time::Duration;

use anyhow::{bail, Error, Result};
use async_trait::async_trait;

use junowen_lib::{
    connection::signaling::{
        socket::{OfferResponse, SignalingSocket},
        CompressedSdp,
    },
    signaling_server::reserved_room::{
        GetReservedRoomResponse, PostReservedRoomSpectateRequestBody,
        PostReservedRoomSpectateResponse,
    },
};
use thiserror::Error;
use tokio::sync::watch;
use tracing::info;

use crate::in_game_lobby::waiting_for_match::socket::{retry_after, sleep_or_abort};

#[derive(Error, Debug)]
pub enum SignalingServerReservedRoomSpectatorSocketError {
    #[error("room not found")]
    RoomNotFound,
    #[error("match is not started")]
    MatchIsNotStarted,
}

pub struct SignalingServerReservedRoomSpectatorSocket {
    client: reqwest::Client,
    resource_url: String,
    abort_rx: watch::Receiver<bool>,
}

impl SignalingServerReservedRoomSpectatorSocket {
    pub fn new(origin: String, room_name: String, abort_rx: watch::Receiver<bool>) -> Self {
        Self {
            client: reqwest::Client::new(),
            resource_url: format!("{}/reserved-room/{}", origin, room_name),
            abort_rx,
        }
    }
}

#[async_trait]
impl SignalingSocket for SignalingServerReservedRoomSpectatorSocket {
    fn timeout() -> Duration {
        Duration::from_secs(10)
    }

    async fn offer(&mut self, _desc: CompressedSdp) -> Result<OfferResponse> {
        loop {
            info!("GET {}", self.resource_url);
            let res = self.client.get(&self.resource_url).send().await?;
            info!("{:?}", res);
            let retry_after = retry_after(&res)
                .ok_or_else(|| Error::msg("retry-after header not found in response"))?;
            let res =
                GetReservedRoomResponse::parse(res.status(), res.text().await.ok().as_deref())?;
            match res {
                GetReservedRoomResponse::NotFound => {
                    bail!(SignalingServerReservedRoomSpectatorSocketError::RoomNotFound);
                }
                GetReservedRoomResponse::Ok(body) => {
                    if body.opponent_offer().is_some() {
                        bail!(SignalingServerReservedRoomSpectatorSocketError::MatchIsNotStarted);
                    }
                    if let Some(spectator_offer) = body.into_spectator_offer() {
                        return Ok(OfferResponse::Offer(spectator_offer));
                    };
                    sleep_or_abort(retry_after, &mut self.abort_rx).await?;
                    continue;
                }
            };
        }
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        let url = format!("{}/spectate", self.resource_url);
        let json = PostReservedRoomSpectateRequestBody::new(desc);
        loop {
            let res = self.client.post(&url).json(&json).send().await?;
            let retry_after = retry_after(&res)
                .ok_or_else(|| Error::msg("retry-after header not found in response"))?;
            let res = PostReservedRoomSpectateResponse::parse(res.status())?;
            match res {
                PostReservedRoomSpectateResponse::Ok => return Ok(()),
                PostReservedRoomSpectateResponse::Conflict => {
                    sleep_or_abort(retry_after, &mut self.abort_rx).await?;
                }
            }
        }
    }
}
