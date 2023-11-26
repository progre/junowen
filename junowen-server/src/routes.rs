mod custom;
mod reserved_room;
mod room_utils;

use std::hash::{DefaultHasher, Hash, Hasher};

use anyhow::{bail, Result};
use base_custom::BaseCustom;
use lambda_http::{
    http::{header::RETRY_AFTER, StatusCode},
    Body, IntoResponse, Request, Response,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use tracing::{info_span, trace, Instrument};

use crate::{database::Database, routes::room_utils::RETRY_AFTER_INTERVAL_SEC};

static BASE_YOTEICHI_MOD: Lazy<BaseCustom<char>> = Lazy::new(|| {
    const CHARS: &str = concat!(
        "ー",
        "あいうえお",
        "かがきぎくぐけげこご",
        "さざしじすずせぜそぞ",
        "ただちぢつづてでとど",
        "なにぬねの",
        "はばぱひびぴふぶぷへべぺほぼぽ",
        "まみむめも",
        "やゆよ",
        "らりるれろ",
        "わゐゑをん",
        "ゔ",
    );
    base_custom::BaseCustom::<char>::new(CHARS.chars().collect())
});

fn base_yoteichi_mod(input_val: u64) -> String {
    BASE_YOTEICHI_MOD
        .gen(input_val)
        .chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn try_parse<'a, T>(body: &'a Body) -> anyhow::Result<T>
where
    T: Deserialize<'a>,
{
    let Body::Text(body) = body else {
        bail!("Not text");
    };
    serde_json::from_str(body.as_str()).map_err(|err| err.into())
}

fn to_response(status_code: StatusCode, body: impl Into<Body>) -> Response<Body> {
    Response::builder()
        .status(status_code)
        .header(RETRY_AFTER, RETRY_AFTER_INTERVAL_SEC)
        .body(body.into())
        .unwrap()
}

fn ip_hash(req: &Request) -> u64 {
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|x| x.to_str().ok())
        .unwrap_or_default();
    let mut s = DefaultHasher::new();
    ip.hash(&mut s);
    s.finish()
}

pub async fn routes(req: &Request, db: &impl Database) -> Result<impl IntoResponse> {
    trace!("{:?}", req);

    if let Some(relative_uri) = req.uri().path().strip_prefix("/custom/") {
        return custom::route(relative_uri, req, db)
            .instrument(info_span!("req", ip_hash = base_yoteichi_mod(ip_hash(req))))
            .await;
    }
    if let Some(relative_uri) = req.uri().path().strip_prefix("/reserved-room/") {
        return reserved_room::route(relative_uri, req, db)
            .instrument(info_span!("req", ip_hash = base_yoteichi_mod(ip_hash(req))))
            .await;
    }
    Ok(to_response(StatusCode::NOT_FOUND, Body::Empty))
}
