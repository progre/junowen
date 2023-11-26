use anyhow::{anyhow, bail, Result};
use derive_new::new;
use getset::Getters;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::connection::signaling::CompressedSdp;

#[derive(Deserialize, Serialize, Getters, new)]
pub struct RequestBody {
    #[get = "pub"]
    offer: CompressedSdp,
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct ResponseWaitingBody {
    key: String,
}

impl ResponseWaitingBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug, Deserialize, Serialize, new)]
pub struct ResponseAnswerBody {
    answer: CompressedSdp,
}

impl ResponseAnswerBody {
    pub fn into_answer(self) -> CompressedSdp {
        self.answer
    }
}

#[derive(Debug)]
pub enum Response<T> {
    CreatedWithKey {
        retry_after: u32,
        body: ResponseWaitingBody,
    },
    CreatedWithAnswer {
        retry_after: u32,
        body: ResponseAnswerBody,
    },
    Conflict {
        retry_after: u32,
        body: T,
    },
}

impl<'a, T> Response<T>
where
    T: Deserialize<'a>,
{
    pub fn created_with_key(retry_after: u32, body: ResponseWaitingBody) -> Self {
        Self::CreatedWithKey { retry_after, body }
    }
    pub fn created_with_answer(retry_after: u32, body: ResponseAnswerBody) -> Self {
        Self::CreatedWithAnswer { retry_after, body }
    }
    pub fn conflict(retry_after: u32, body: T) -> Self {
        Self::Conflict { retry_after, body }
    }

    pub fn parse(status: StatusCode, retry_after: Option<u32>, text: &'a str) -> Result<Self> {
        match status {
            StatusCode::CREATED => {
                if let Ok(res) = serde_json::from_str::<ResponseWaitingBody>(text) {
                    return Ok(Self::CreatedWithKey {
                        retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
                        body: res,
                    });
                }
                if let Ok(res) = serde_json::from_str::<ResponseAnswerBody>(text) {
                    return Ok(Self::CreatedWithAnswer {
                        retry_after: retry_after.ok_or_else(|| anyhow!("invalid response"))?,
                        body: res,
                    });
                }
            }
            StatusCode::CONFLICT => {
                if let Ok(res) = serde_json::from_str(text) {
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
            Response::CreatedWithKey { .. } => StatusCode::CREATED,
            Response::CreatedWithAnswer { .. } => StatusCode::CREATED,
            Response::Conflict { .. } => StatusCode::CONFLICT,
        }
    }

    pub fn retry_after(&self) -> u32 {
        match self {
            Response::CreatedWithKey { retry_after, .. } => *retry_after,
            Response::CreatedWithAnswer { retry_after, .. } => *retry_after,
            Response::Conflict { retry_after, .. } => *retry_after,
        }
    }
}
