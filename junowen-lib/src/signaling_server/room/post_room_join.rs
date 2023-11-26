use anyhow::{bail, Result};
use derive_new::new;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::connection::signaling::CompressedSdp;

#[derive(Deserialize, Serialize, new)]
pub struct RequestBody {
    answer: CompressedSdp,
}

impl RequestBody {
    pub fn into_answer(self) -> CompressedSdp {
        self.answer
    }
}

pub enum Response {
    Ok,
    Conflict,
}

impl Response {
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
            Response::Ok => StatusCode::CREATED,
            Response::Conflict => StatusCode::CONFLICT,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Response::Ok => StatusCode::OK,
            Response::Conflict => StatusCode::CONFLICT,
        }
    }
}
