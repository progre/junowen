mod custom;

use std::hash::{DefaultHasher, Hash, Hasher};

use anyhow::{bail, Result};
use lambda_http::{
    http::{header::RETRY_AFTER, StatusCode},
    Body, IntoResponse, Request, Response,
};
use regex::Regex;
use serde::Deserialize;
use tracing::{info_span, trace, Instrument};

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

fn base_yoteichi_mod(input_val: u64) -> String {
    let chars = concat!(
        "あ い う え お ",
        "か が き ぎ く ぐ け げ こ ご ",
        "さ ざ し じ す ず せ ぜ そ ぞ ",
        "た だ ち ぢ つ づ て で と ど ",
        "な に ぬ ね の ",
        "は ば ぱ ひ び ぴ ふ ぶ ぷ へ べ ぺ ほ ぼ ぽ ",
        "ま み む め も ",
        "や ゆ よ ",
        "ら り る れ ろ ",
        "わ ゐ ゑ を ん ",
        "きゃ きゅ きょ ぎゃ ぎゅ ぎょ ",
        "しゃ しゅ しょ じゃ じゅ じょ ",
        "ちゃ ちゅ ちょ ぢゃ ぢゅ ぢょ ",
        "にゃ にゅ にょ ",
        "ひゃ ひゅ ひょ びゃ びゅ びょ ぴゃ ぴゅ ぴょ ",
        "みゃ みゅ みょ ",
        "りゃ りゅ りょ ",
        "っか っき っきゃ っきゅ っきょ っく っけ っこ ",
        "っさ っし っしゃ っしゅ っしょ っす っせ っそ ",
        "った っち っちゃ っちゅ っちょ っつ って っと っど ",
        "っぱ っぴ っぴゃ っぴゅ っぴょ っぷ っぺ っぽ ",
        "うぃ うぇ うぉ ",
        "ゔぁ ゔぃ ゔ ゔぇ ゔぉ ",
        "しぇ じぇ ちぇ びぇ ぴぇ りぇ ",
        "つぁ つぃ つぇ つぉ ",
        "てぃ でぃ とぅ どぅ ",
        "ふぁ ふぃ ふぇ ふぉ ふゅ ",
        "ー"
    );
    let base = base_custom::BaseCustom::<String>::new(chars, Some(' '));
    let mut encoded = base.gen(input_val).replace(' ', "");
    if Regex::new(r"^お(:?[っー])").unwrap().is_match(&encoded) {
        encoded = format!("お{}", encoded);
    }
    let encoded = Regex::new(r"^っ")
        .unwrap()
        .replace(&encoded, "おっ")
        .to_string();
    Regex::new(r"^ー")
        .unwrap()
        .replace(&encoded, "おー")
        .to_string()
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

pub async fn app(req: &Request, db: &impl Database) -> Result<impl IntoResponse> {
    trace!("{:?}", req);

    if let Some(relative_uri) = req.uri().path().strip_prefix("/custom/") {
        return custom(relative_uri, req, db)
            .instrument(info_span!("req", ip_hash = base_yoteichi_mod(ip_hash(req))))
            .await;
    }
    Ok(to_response(StatusCode::NOT_FOUND, None, Body::Empty))
}
