use anyhow::{anyhow, bail, Result};
use derive_new::new;
use getset::Getters;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::connection::signaling::CompressedSdp;

#[derive(Debug, Deserialize, Serialize, new)]
pub struct FindAnswerResponseWaitingBody {
    key: String,
}

impl FindAnswerResponseWaitingBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct FindAnswerResponseAnswerBody {
    answer: CompressedSdp,
}

impl FindAnswerResponseAnswerBody {
    pub fn into_answer(self) -> CompressedSdp {
        self.answer
    }
}

#[derive(Deserialize, Serialize, Getters, new)]
pub struct PutOfferRequestBody {
    #[get = "pub"]
    offer: CompressedSdp,
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PutOfferResponseConflictBody {
    offer: CompressedSdp,
}

impl PutOfferResponseConflictBody {
    pub fn into_offer(self) -> CompressedSdp {
        self.offer
    }
}

#[derive(Debug)]
pub enum PutOfferResponse {
    CreatedWithKey {
        retry_after: u32,
        body: FindAnswerResponseWaitingBody,
    },
    CreatedWithAnswer {
        retry_after: u32,
        body: FindAnswerResponseAnswerBody,
    },
    Conflict {
        retry_after: u32,
        body: PutOfferResponseConflictBody,
    },
}

impl PutOfferResponse {
    pub fn created_with_key(retry_after: u32, body: FindAnswerResponseWaitingBody) -> Self {
        Self::CreatedWithKey { retry_after, body }
    }
    pub fn created_with_answer(retry_after: u32, body: FindAnswerResponseAnswerBody) -> Self {
        Self::CreatedWithAnswer { retry_after, body }
    }
    pub fn conflict(retry_after: u32, body: PutOfferResponseConflictBody) -> Self {
        Self::Conflict { retry_after, body }
    }

    pub fn parse(status: StatusCode, retry_after: Option<u32>, text: &str) -> Result<Self> {
        match status {
            StatusCode::CREATED => {
                if let Ok(res) = serde_json::from_str::<FindAnswerResponseWaitingBody>(text) {
                    return Ok(Self::CreatedWithKey {
                        retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
                        body: res,
                    });
                }
                if let Ok(res) = serde_json::from_str::<FindAnswerResponseAnswerBody>(text) {
                    return Ok(Self::CreatedWithAnswer {
                        retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
                        body: res,
                    });
                }
            }
            StatusCode::CONFLICT => {
                if let Ok(res) = serde_json::from_str::<PutOfferResponseConflictBody>(text) {
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
            PutOfferResponse::CreatedWithKey { .. } => StatusCode::CREATED,
            PutOfferResponse::CreatedWithAnswer { .. } => StatusCode::CREATED,
            PutOfferResponse::Conflict { .. } => StatusCode::CONFLICT,
        }
    }

    pub fn retry_after(&self) -> u32 {
        match self {
            PutOfferResponse::CreatedWithKey { retry_after, .. } => *retry_after,
            PutOfferResponse::CreatedWithAnswer { retry_after, .. } => *retry_after,
            PutOfferResponse::Conflict { retry_after, .. } => *retry_after,
        }
    }
}

#[derive(Deserialize, Serialize, Getters, new)]
pub struct DeleteOfferRequestBody {
    key: String,
}

impl DeleteOfferRequestBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug)]
pub enum DeleteOfferResponse {
    BadRequest,
    NoContent,
}

impl DeleteOfferResponse {
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
pub struct PostOfferKeepRequestBody {
    key: String,
}

impl PostOfferKeepRequestBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug)]
pub enum PostOfferKeepResponse {
    BadRequest,
    NoContent { retry_after: u32 },
    Ok(FindAnswerResponseAnswerBody),
}

impl PostOfferKeepResponse {
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
            PostOfferKeepResponse::BadRequest => None,
            PostOfferKeepResponse::NoContent { retry_after } => Some(*retry_after),
            PostOfferKeepResponse::Ok(_) => None,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            PostOfferKeepResponse::BadRequest => StatusCode::BAD_REQUEST,
            PostOfferKeepResponse::NoContent { .. } => StatusCode::NO_CONTENT,
            PostOfferKeepResponse::Ok(_) => StatusCode::OK,
        }
    }
}

#[derive(Deserialize, Serialize, new)]
pub struct PostAnswerRequestBody {
    answer: CompressedSdp,
}

impl PostAnswerRequestBody {
    pub fn into_answer(self) -> CompressedSdp {
        self.answer
    }
}

pub enum PostAnswerResponse {
    Created,
    Conflict,
}

impl PostAnswerResponse {
    pub fn parse(status: StatusCode) -> Result<Self> {
        match status {
            StatusCode::CREATED => Ok(Self::Created),
            StatusCode::CONFLICT => Ok(Self::Conflict),
            _ => bail!("invalid response"),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            PostAnswerResponse::Created => StatusCode::CREATED,
            PostAnswerResponse::Conflict => StatusCode::CONFLICT,
        }
    }
}
