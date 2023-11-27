use anyhow::bail;
use anyhow::Result;
use derive_new::new;
use getset::Getters;
use http::StatusCode;
use serde::Deserialize;
use serde::Serialize;

use crate::connection::signaling::CompressedSdp;

use super::room::PostRoomKeepResponse;
use super::room::PutRoomResponse;

// PUT /reserved-room/{name}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PutReservedRoomResponseConflictBody {
    opponent_offer: Option<CompressedSdp>,
}

impl PutReservedRoomResponseConflictBody {
    pub fn into_offer(self) -> Option<CompressedSdp> {
        self.opponent_offer
    }
}

pub type PutReservedRoomResponse = PutRoomResponse<PutReservedRoomResponseConflictBody>;

// GET /reserved-room/{name}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct GetReservedRoomResponseOkBody {
    opponent_offer: Option<CompressedSdp>,
    spectator_offer: Option<CompressedSdp>,
}

impl GetReservedRoomResponseOkBody {
    pub fn opponent_offer(&self) -> Option<&CompressedSdp> {
        self.opponent_offer.as_ref()
    }

    pub fn into_spectator_offer(self) -> Option<CompressedSdp> {
        self.spectator_offer
    }
}

pub enum GetReservedRoomResponse {
    Ok(GetReservedRoomResponseOkBody),
    NotFound,
}

impl GetReservedRoomResponse {
    pub fn parse(status: StatusCode, text: Option<&str>) -> Result<Self> {
        match (status, text) {
            (StatusCode::OK, Some(text)) => {
                if let Ok(body) = serde_json::from_str::<GetReservedRoomResponseOkBody>(text) {
                    return Ok(Self::Ok(body));
                }
            }
            (StatusCode::NOT_FOUND, _) => return Ok(Self::NotFound),
            _ => {}
        }
        bail!("invalid response")
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Ok(_) => StatusCode::OK,
            Self::NotFound => StatusCode::NOT_FOUND,
        }
    }

    pub fn to_body(&self) -> Option<String> {
        match self {
            Self::Ok(body) => Some(serde_json::to_string(&body).unwrap()),
            Self::NotFound => None,
        }
    }
}

// POST /reserved-room/{name}/keep

#[derive(Deserialize, Serialize, Getters, new)]
pub struct PostReservedRoomKeepRequestBody {
    key: String,
    spectator_offer: Option<CompressedSdp>,
}

impl PostReservedRoomKeepRequestBody {
    pub fn into_inner(self) -> (String, Option<CompressedSdp>) {
        (self.key, self.spectator_offer)
    }
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PostReservedRoomKeepResponseOkOpponentAnswerBody {
    opponent_answer: CompressedSdp,
}

impl PostReservedRoomKeepResponseOkOpponentAnswerBody {
    pub fn into_opponent_answer(self) -> CompressedSdp {
        self.opponent_answer
    }
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PostReservedRoomKeepResponseOkSpectatorAnswerBody {
    spectator_answer: CompressedSdp,
}

impl PostReservedRoomKeepResponseOkSpectatorAnswerBody {
    pub fn into_spectator_answer(self) -> CompressedSdp {
        self.spectator_answer
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PostReservedRoomKeepResponseOkBody {
    OpponentAnswer(PostReservedRoomKeepResponseOkOpponentAnswerBody),
    SpectatorAnswer(PostReservedRoomKeepResponseOkSpectatorAnswerBody),
}

impl From<PostReservedRoomKeepResponseOkOpponentAnswerBody> for PostReservedRoomKeepResponseOkBody {
    fn from(body: PostReservedRoomKeepResponseOkOpponentAnswerBody) -> Self {
        Self::OpponentAnswer(body)
    }
}

impl From<PostReservedRoomKeepResponseOkSpectatorAnswerBody>
    for PostReservedRoomKeepResponseOkBody
{
    fn from(body: PostReservedRoomKeepResponseOkSpectatorAnswerBody) -> Self {
        Self::SpectatorAnswer(body)
    }
}

pub type PostReservedRoomKeepResponse = PostRoomKeepResponse<PostReservedRoomKeepResponseOkBody>;

impl From<PostReservedRoomKeepResponseOkBody> for PostReservedRoomKeepResponse {
    fn from(body: PostReservedRoomKeepResponseOkBody) -> Self {
        Self::Ok(body)
    }
}

// POST /reserved-room/{name}/join

pub use super::room::PostRoomJoinRequestBody as PostReservedRoomSpectateRequestBody;
pub use super::room::PostRoomJoinResponse as PostReservedRoomSpectateResponse;
