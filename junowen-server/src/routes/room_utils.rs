use std::time::{SystemTime, UNIX_EPOCH};

use junowen_lib::signaling_server::room::{PostRoomKeepResponse, PutRoomResponse};
use lambda_http::{Body, Response};
use serde::{Deserialize, Serialize};

use super::to_response;

const OFFER_TTL_DURATION_SEC: u64 = 10;
pub const RETRY_AFTER_INTERVAL_SEC: u32 = 3;

pub fn now_sec() -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    now.as_secs()
}

pub fn ttl_sec(now_sec: u64) -> u64 {
    now_sec + OFFER_TTL_DURATION_SEC
}

pub fn from_put_room_response<'a, T>(value: PutRoomResponse<T>) -> Response<Body>
where
    T: Deserialize<'a> + Serialize,
{
    let status_code = value.status_code();
    let body = match value {
        PutRoomResponse::CreatedWithKey { body, .. } => {
            Body::Text(serde_json::to_string(&body).unwrap())
        }
        PutRoomResponse::CreatedWithAnswer { body, .. } => {
            Body::Text(serde_json::to_string(&body).unwrap())
        }
        PutRoomResponse::Conflict { body, .. } => Body::Text(serde_json::to_string(&body).unwrap()),
    };
    to_response(status_code, body)
}

pub fn from_post_room_keep_response<'a, T>(value: PostRoomKeepResponse<T>) -> Response<Body>
where
    T: Deserialize<'a> + Serialize,
{
    let status_code = value.status_code();
    let body = match value {
        PostRoomKeepResponse::BadRequest => Body::Empty,
        PostRoomKeepResponse::NoContent { .. } => Body::Empty,
        PostRoomKeepResponse::Ok(body) => Body::Text(serde_json::to_string(&body).unwrap()),
    };
    to_response(status_code, body)
}
