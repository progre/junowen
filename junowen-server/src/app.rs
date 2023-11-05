mod custom;

use anyhow::{bail, Result};
use lambda_http::{
    http::{header::RETRY_AFTER, StatusCode},
    Body, IntoResponse, Request, Response,
};
use serde::Deserialize;
use tracing::trace;

use crate::{app::custom::custom, database::Database};

fn try_parse<'a, T>(body: &'a Body) -> anyhow::Result<T>
where
    T: Deserialize<'a>,
{
    let Body::Text(body) = body else {
        bail!("Not text");
    };
    serde_json::from_str(body.as_str()).map_err(|err| err.into())
}

fn to_response(
    status_code: StatusCode,
    retry_after: Option<u32>,
    body: impl Into<Body>,
) -> Response<Body> {
    let mut builder = Response::builder().status(status_code);
    if let Some(retry_after) = retry_after {
        builder = builder.header(RETRY_AFTER, retry_after);
    }
    builder.body(body.into()).unwrap()
}

pub async fn app(req: &Request, db: &impl Database) -> Result<impl IntoResponse> {
    trace!("{:?}", req);
    if let Some(relative_uri) = req.uri().path().strip_prefix("/custom/") {
        return custom(relative_uri, req, db).await;
    }
    Ok(to_response(StatusCode::NOT_FOUND, None, Body::Empty))
}
