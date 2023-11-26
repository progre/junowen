use anyhow::{bail, Result};
use derive_new::new;
use getset::Getters;
use http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Getters, new)]
pub struct RequestBody {
    key: String,
}

impl RequestBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

#[derive(Debug)]
pub enum Response {
    BadRequest,
    NoContent,
}

impl Response {
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
