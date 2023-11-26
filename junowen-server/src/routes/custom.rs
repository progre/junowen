use anyhow::{bail, Result};
use junowen_lib::signaling_server::{
    custom::{
        PostSharedRoomKeepRequestBody, PostSharedRoomKeepResponse, PutSharedRoomResponse,
        PutSharedRoomResponseConflictBody,
    },
    room::{
        DeleteRoomRequestBody, DeleteRoomResponse, PostRoomJoinRequestBody, PostRoomJoinResponse,
        PostRoomKeepResponse, PutRoomRequestBody, PutRoomResponseAnswerBody,
        PutRoomResponseWaitingBody,
    },
};
use lambda_http::{
    http::{Method, StatusCode},
    Body, Request, Response,
};
use regex::Regex;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    database::{PutError, SharedRoom, SharedRoomOpponentAnswer, SharedRoomTables},
    routes::room_utils::{now_sec, ttl_sec, RETRY_AFTER_INTERVAL_SEC},
};

use super::{
    room_utils::{from_post_room_keep_response, from_put_room_response},
    to_response, try_parse,
};

async fn find_valid_room(
    db: &impl SharedRoomTables,
    now_sec: u64,
    name: String,
) -> Result<Option<SharedRoom>> {
    let Some(offer) = db.find_room(name.to_owned()).await? else {
        return Ok(None);
    };
    if !offer.is_expired(now_sec) {
        return Ok(Some(offer));
    }
    db.remove_room(offer.name().clone(), None).await?;
    db.remove_room_opponent_answer(offer.name().clone()).await?;
    Ok(None)
}

async fn find_guest(
    db: &impl SharedRoomTables,
    name: String,
) -> Result<Option<SharedRoomOpponentAnswer>> {
    let Some(answer) = db.remove_room_opponent_answer(name.to_owned()).await? else {
        return Ok(None);
    };
    db.remove_room(name.to_owned(), None).await?;
    Ok(Some(answer))
}

async fn put_room(
    db: &impl SharedRoomTables,
    name: &str,
    body: PutRoomRequestBody,
) -> Result<PutSharedRoomResponse> {
    let now_sec = now_sec();
    let key = Uuid::new_v4().to_string();
    let room = SharedRoom::new(
        name.to_owned(),
        key.clone(),
        body.offer().clone(),
        ttl_sec(now_sec),
    );
    for retry in 0.. {
        if let Some(room) = find_valid_room(db, now_sec, name.to_owned()).await? {
            let body = PutSharedRoomResponseConflictBody::new(room.into_sdp());
            let response = PutSharedRoomResponse::conflict(RETRY_AFTER_INTERVAL_SEC, body);
            return Ok(response);
        }
        match db.put_room(room.clone()).await {
            Ok(()) => break,
            Err(PutError::Conflict) => {
                if retry >= 2 {
                    panic!();
                }
                continue;
            }
            Err(PutError::Unknown(err)) => bail!("{:?}", err),
        }
    }
    info!("[Shared Room] Created: {}", name);
    Ok(
        if let Some(answer) = find_guest(db, name.to_owned()).await? {
            let body = PutRoomResponseAnswerBody::new(answer.into_sdp());
            PutSharedRoomResponse::created_with_answer(RETRY_AFTER_INTERVAL_SEC, body)
        } else {
            let body = PutRoomResponseWaitingBody::new(key);
            PutSharedRoomResponse::created_with_key(RETRY_AFTER_INTERVAL_SEC, body)
        },
    )
}

async fn post_room_keep(
    db: &impl SharedRoomTables,
    name: &str,
    body: PostSharedRoomKeepRequestBody,
) -> Result<PostSharedRoomKeepResponse> {
    let key = body.into_key();
    if Uuid::parse_str(&key).is_err() {
        return Ok(PostRoomKeepResponse::BadRequest);
    }
    if !db
        .keep_room(name.to_owned(), key, ttl_sec(now_sec()))
        .await?
    {
        return Ok(PostRoomKeepResponse::BadRequest);
    }
    Ok(
        if let Some(answer) = find_guest(db, name.to_owned()).await? {
            PostRoomKeepResponse::Ok(PutRoomResponseAnswerBody::new(answer.into_sdp()))
        } else {
            let retry_after = RETRY_AFTER_INTERVAL_SEC;
            PostRoomKeepResponse::NoContent { retry_after }
        },
    )
}

async fn delete_room(
    db: &impl SharedRoomTables,
    name: &str,
    body: DeleteRoomRequestBody,
) -> Result<DeleteRoomResponse> {
    if !db
        .remove_room(name.to_owned(), Some(body.into_key()))
        .await?
    {
        Ok(DeleteRoomResponse::BadRequest)
    } else {
        db.remove_room_opponent_answer(name.to_owned()).await?;
        info!("[Shared Room] Removed: {}", name);
        Ok(DeleteRoomResponse::NoContent)
    }
}

async fn post_room_join(
    db: &impl SharedRoomTables,
    name: &str,
    body: PostRoomJoinRequestBody,
) -> Result<PostRoomJoinResponse> {
    let answer =
        SharedRoomOpponentAnswer::new(name.to_owned(), body.into_answer(), ttl_sec(now_sec()));
    match db.put_room_opponent_answer(answer).await {
        Ok(()) => {
            info!("[Shared Room] Answered: {}", name);
            Ok(PostRoomJoinResponse::Ok)
        }
        Err(PutError::Conflict) => Ok(PostRoomJoinResponse::Conflict),
        Err(PutError::Unknown(err)) => Err(err),
    }
}

pub async fn route(
    relative_uri: &str,
    req: &Request,
    db: &impl SharedRoomTables,
) -> Result<Response<Body>> {
    let regex = Regex::new(r"^([^/]+)$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        return Ok(match *req.method() {
            Method::PUT => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = put_room(db, &c[1], body).await?;
                    from_put_room_response(res)
                }
            },
            Method::DELETE => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = delete_room(db, &c[1], body).await?;
                    to_response(res.status_code(), Body::Empty)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, Body::Empty),
        });
    }
    let regex = Regex::new(r"^([^/]+)/join$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        return Ok(match *req.method() {
            Method::POST => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = post_room_join(db, &c[1], body).await?;
                    to_response(res.status_code_old(), Body::Empty)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, Body::Empty),
        });
    }
    let regex = Regex::new(r"^([^/]+)/keep$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        return Ok(match *req.method() {
            Method::POST => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = post_room_keep(db, &c[1], body).await?;
                    from_post_room_keep_response(res)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, Body::Empty),
        });
    }
    Ok(to_response(StatusCode::NOT_FOUND, Body::Empty))
}
