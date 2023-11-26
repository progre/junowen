use anyhow::{anyhow, bail, Result};
use http::StatusCode;
use serde::Deserialize;

#[derive(Debug)]
pub enum Response<T> {
    BadRequest,
    NoContent { retry_after: u32 },
    Ok(T),
}

impl<'a, T> Response<T>
where
    T: Deserialize<'a>,
{
    pub fn parse(
        status: StatusCode,
        retry_after: Option<u32>,
        text: Option<&'a str>,
    ) -> Result<Self> {
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
            Response::BadRequest => None,
            Response::NoContent { retry_after } => Some(*retry_after),
            Response::Ok(_) => None,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Response::BadRequest => StatusCode::BAD_REQUEST,
            Response::NoContent { .. } => StatusCode::NO_CONTENT,
            Response::Ok(_) => StatusCode::OK,
        }
    }
}
