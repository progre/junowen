use anyhow::{anyhow, bail, Result};
use derive_new::new;
use getset::Getters;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::connection::signaling::CompressedSdp;

#[derive(Deserialize, Serialize, Getters, new)]
pub struct PutRoomRequestBody {
    #[get = "pub"]
    offer: CompressedSdp,
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PutRoomResponseWaitingBody {
    key: String,
}

impl PutRoomResponseWaitingBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PutRoomResponseAnswerBody {
    answer: CompressedSdp,
}

impl PutRoomResponseAnswerBody {
    pub fn into_answer(self) -> CompressedSdp {
        self.answer
    }
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PutRoomResponseConflictBody {
    offer: CompressedSdp,
}

impl PutRoomResponseConflictBody {
    pub fn into_offer(self) -> CompressedSdp {
        self.offer
    }
}

#[derive(Debug)]
pub enum PutRoomResponse {
    CreatedWithKey {
        retry_after: u32,
        body: PutRoomResponseWaitingBody,
    },
    CreatedWithAnswer {
        retry_after: u32,
        body: PutRoomResponseAnswerBody,
    },
    Conflict {
        retry_after: u32,
        body: PutRoomResponseConflictBody,
    },
}

impl PutRoomResponse {
    pub fn created_with_key(retry_after: u32, body: PutRoomResponseWaitingBody) -> Self {
        Self::CreatedWithKey { retry_after, body }
    }
    pub fn created_with_answer(retry_after: u32, body: PutRoomResponseAnswerBody) -> Self {
        Self::CreatedWithAnswer { retry_after, body }
    }
    pub fn conflict(retry_after: u32, body: PutRoomResponseConflictBody) -> Self {
        Self::Conflict { retry_after, body }
    }

    pub fn parse(status: StatusCode, retry_after: Option<u32>, text: &str) -> Result<Self> {
        match status {
            StatusCode::CREATED => {
                if let Ok(res) = serde_json::from_str::<PutRoomResponseWaitingBody>(text) {
                    return Ok(Self::CreatedWithKey {
                        retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
                        body: res,
                    });
                }
                if let Ok(res) = serde_json::from_str::<PutRoomResponseAnswerBody>(text) {
                    return Ok(Self::CreatedWithAnswer {
                        retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
                        body: res,
                    });
                }
            }
            StatusCode::CONFLICT => {
                if let Ok(res) = serde_json::from_str::<PutRoomResponseConflictBody>(text) {
                    return Ok(Self::Conflict {
                        retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
                        body: res,
                    });
                }
            }
            _ => {}
        }
        bail!("invalid response")
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            PutRoomResponse::CreatedWithKey { .. } => StatusCode::CREATED,
            PutRoomResponse::CreatedWithAnswer { .. } => StatusCode::CREATED,
            PutRoomResponse::Conflict { .. } => StatusCode::CONFLICT,
        }
    }

    pub fn retry_after(&self) -> u32 {
        match self {
            PutRoomResponse::CreatedWithKey { retry_after, .. } => *retry_after,
            PutRoomResponse::CreatedWithAnswer { retry_after, .. } => *retry_after,
            PutRoomResponse::Conflict { retry_after, .. } => *retry_after,
        }
    }
}

#[derive(Deserialize, Serialize, Getters, new)]
pub struct DeleteRoomRequestBody {
    key: String,
}

impl DeleteRoomRequestBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug)]
pub enum DeleteRoomResponse {
    BadRequest,
    NoContent,
}

impl DeleteRoomResponse {
    pub fn parse(status: StatusCode) -> Result<Self> {
        match status {
            StatusCode::BAD_REQUEST => Ok(Self::BadRequest),
            StatusCode::NO_CONTENT => Ok(Self::NoContent),
            _ => bail!("invalid response"),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::NoContent => StatusCode::NO_CONTENT,
        }
    }
}

#[derive(Deserialize, Serialize, Getters, new)]
pub struct PostRoomKeepRequestBody {
    key: String,
}

impl PostRoomKeepRequestBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug)]
pub enum PostRoomKeepResponse {
    BadRequest,
    NoContent { retry_after: u32 },
    Ok(PutRoomResponseAnswerBody),
}

impl PostRoomKeepResponse {
    pub fn parse(status: StatusCode, retry_after: Option<u32>, text: Option<&str>) -> Result<Self> {
        match status {
            StatusCode::BAD_REQUEST => Ok(Self::BadRequest),
            StatusCode::NO_CONTENT => Ok(Self::NoContent {
                retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
            }),
            StatusCode::OK => Ok(Self::Ok(serde_json::from_str(
                text.ok_or_else(|| anyhow!("invalid response"))?,
            )?)),
            _ => bail!("invalid response"),
        }
    }

    pub fn retry_after(&self) -> Option<u32> {
        match self {
            PostRoomKeepResponse::BadRequest => None,
            PostRoomKeepResponse::NoContent { retry_after } => Some(*retry_after),
            PostRoomKeepResponse::Ok(_) => None,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            PostRoomKeepResponse::BadRequest => StatusCode::BAD_REQUEST,
            PostRoomKeepResponse::NoContent { .. } => StatusCode::NO_CONTENT,
            PostRoomKeepResponse::Ok(_) => StatusCode::OK,
        }
    }
}

#[derive(Deserialize, Serialize, new)]
pub struct PostRoomJoinRequestBody {
    answer: CompressedSdp,
}

impl PostRoomJoinRequestBody {
    pub fn into_answer(self) -> CompressedSdp {
        self.answer
    }
}

pub enum PostRoomJoinResponse {
    Ok,
    Conflict,
}

impl PostRoomJoinResponse {
    pub fn parse(status: StatusCode) -> Result<Self> {
        match status {
            StatusCode::OK => Ok(Self::Ok),
            StatusCode::CREATED => Ok(Self::Ok),
            StatusCode::CONFLICT => Ok(Self::Conflict),
            _ => bail!("invalid response"),
        }
    }

    pub fn status_code_old(&self) -> StatusCode {
        match self {
            PostRoomJoinResponse::Ok => StatusCode::CREATED,
            PostRoomJoinResponse::Conflict => StatusCode::CONFLICT,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            PostRoomJoinResponse::Ok => StatusCode::OK,
            PostRoomJoinResponse::Conflict => StatusCode::CONFLICT,
        }
    }
}
