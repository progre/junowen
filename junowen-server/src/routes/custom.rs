use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Result};
use junowen_lib::signaling_server::custom::{
    DeleteOfferRequestBody, DeleteOfferResponse, FindAnswerResponseAnswerBody,
    FindAnswerResponseWaitingBody, PostAnswerRequestBody, PostAnswerResponse,
    PostOfferKeepRequestBody, PostOfferKeepResponse, PutOfferRequestBody, PutOfferResponse,
    PutOfferResponseConflictBody,
};
use lambda_http::{
    http::{Method, StatusCode},
    Body, Request, Response,
};
use regex::Regex;
use tracing::{debug, info};
use uuid::Uuid;

use crate::database::{Answer, Database, Offer, PutError};

use super::{to_response, try_parse};

const OFFER_TTL_DURATION_SEC: u64 = 10;
const RETRY_AFTER_INTERVAL_SEC: u32 = 3;

fn from_put_offer_response(value: PutOfferResponse) -> Response<Body> {
    let status_code = value.status_code();
    let retry_after = Some(value.retry_after());
    let body = match value {
        PutOfferResponse::CreatedWithKey { body, .. } => {
            Body::Text(serde_json::to_string(&body).unwrap())
        }
        PutOfferResponse::CreatedWithAnswer { body, .. } => {
            Body::Text(serde_json::to_string(&body).unwrap())
        }
        PutOfferResponse::Conflict { body, .. } => {
            Body::Text(serde_json::to_string(&body).unwrap())
        }
    };
    to_response(status_code, retry_after, body)
}

fn from_post_offer_keep_response(value: PostOfferKeepResponse) -> Response<Body> {
    let status_code = value.status_code();
    let retry_after = value.retry_after();
    let body = match value {
        PostOfferKeepResponse::BadRequest => Body::Empty,
        PostOfferKeepResponse::NoContent { .. } => Body::Empty,
        PostOfferKeepResponse::Ok(body) => Body::Text(serde_json::to_string(&body).unwrap()),
    };
    to_response(status_code, retry_after, body)
}

fn now_sec() -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    now.as_secs()
}

fn ttl_sec(now_sec: u64) -> u64 {
    now_sec + OFFER_TTL_DURATION_SEC
}

async fn find_answer_and_remove(db: &impl Database, name: String) -> Result<Option<Answer>> {
    let answer = db.find_answer(name.to_owned()).await?;
    let Some(answer) = answer else {
        return Ok(None);
    };
    db.remove_offer(name.to_owned()).await?;
    db.remove_answer(name).await?;
    Ok(Some(answer))
}

async fn put_offer(
    db: &impl Database,
    name: &str,
    body: PutOfferRequestBody,
) -> Result<PutOfferResponse> {
    let now_sec = now_sec();
    let key = Uuid::new_v4().to_string();
    for retry in 0.. {
        let offer: Option<Offer> = db.find_offer(name.to_owned()).await?;
        if let Some(offer) = offer {
            if !offer.is_expired(now_sec) {
                return Ok(PutOfferResponse::conflict(
                    RETRY_AFTER_INTERVAL_SEC,
                    PutOfferResponseConflictBody::new(offer.into_sdp()),
                ));
            }
            db.remove_offer(offer.name().clone()).await?;
            db.remove_answer(offer.name().clone()).await?;
        };
        let offer = Offer::new(
            name.to_owned(),
            key.clone(),
            body.offer().clone(),
            ttl_sec(now_sec),
        );
        match db.put_offer(offer).await {
            Ok(()) => break,
            Err(PutError::Conflict) => {
                if retry >= 2 {
                    panic!();
                }
                continue;
            }
            Err(err) => bail!("{:?}", err),
        }
    }
    info!("[Shared Room] Created: {}", name);
    let Some(answer) = find_answer_and_remove(db, name.to_owned()).await? else {
        return Ok(PutOfferResponse::created_with_key(
            RETRY_AFTER_INTERVAL_SEC,
            FindAnswerResponseWaitingBody::new(key),
        ));
    };
    Ok(PutOfferResponse::created_with_answer(
        RETRY_AFTER_INTERVAL_SEC,
        FindAnswerResponseAnswerBody::new(answer.into_sdp()),
    ))
}

async fn delete_offer_and_answer(
    db: &impl Database,
    name: &str,
    body: DeleteOfferRequestBody,
) -> Result<DeleteOfferResponse> {
    if !db
        .remove_offer_with_key(name.to_owned(), body.into_key())
        .await?
    {
        return Ok(DeleteOfferResponse::BadRequest);
    }
    db.remove_answer(name.to_owned()).await?;
    info!("[Shared Room] Removed: {}", name);
    Ok(DeleteOfferResponse::NoContent)
}

async fn post_offer_keep(
    db: &impl Database,
    name: &str,
    body: PostOfferKeepRequestBody,
) -> Result<PostOfferKeepResponse> {
    let key = body.into_key();
    if Uuid::parse_str(&key).is_err() {
        return Ok(PostOfferKeepResponse::BadRequest);
    }
    if db
        .keep_offer(name.to_owned(), key, ttl_sec(now_sec()))
        .await?
        .is_none()
    {
        return Ok(PostOfferKeepResponse::BadRequest);
    }
    let Some(answer) = find_answer_and_remove(db, name.to_owned()).await? else {
        return Ok(PostOfferKeepResponse::NoContent {
            retry_after: RETRY_AFTER_INTERVAL_SEC,
        });
    };
    Ok(PostOfferKeepResponse::Ok(
        FindAnswerResponseAnswerBody::new(answer.into_sdp()),
    ))
}

async fn post_answer(
    db: &impl Database,
    name: &str,
    body: PostAnswerRequestBody,
) -> Result<PostAnswerResponse> {
    let answer = Answer::new(name.to_owned(), body.into_answer(), ttl_sec(now_sec()));
    match db.put_answer(answer).await {
        Ok(()) => {
            info!("[Shared Room] Answered: {}", name);
            Ok(PostAnswerResponse::Ok)
        }
        Err(PutError::Conflict) => Ok(PostAnswerResponse::Conflict),
        Err(PutError::Unknown(err)) => Err(err),
    }
}

pub async fn custom(
    relative_uri: &str,
    req: &Request,
    db: &impl Database,
) -> Result<Response<Body>> {
    let regex = Regex::new(r"^([^/]+)$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        return Ok(match *req.method() {
            Method::PUT => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, None, Body::Empty)
                }
                Ok(body) => {
                    let res = put_offer(db, &c[1], body).await?;
                    from_put_offer_response(res)
                }
            },
            Method::DELETE => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, None, Body::Empty)
                }
                Ok(body) => {
                    let res = delete_offer_and_answer(db, &c[1], body).await?;
                    to_response(res.status_code(), None, Body::Empty)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, None, Body::Empty),
        });
    }
    let regex = Regex::new(r"^([^/]+)/join$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        return Ok(match *req.method() {
            Method::POST => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, None, Body::Empty)
                }
                Ok(body) => {
                    let res = post_answer(db, &c[1], body).await?;
                    to_response(res.status_code_old(), None, Body::Empty)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, None, Body::Empty),
        });
    }
    let regex = Regex::new(r"^([^/]+)/keep$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        return Ok(match *req.method() {
            Method::POST => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, None, Body::Empty)
                }
                Ok(body) => {
                    let res = post_offer_keep(db, &c[1], body).await?;
                    from_post_offer_keep_response(res)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, None, Body::Empty),
        });
    }
    Ok(to_response(StatusCode::NOT_FOUND, None, Body::Empty))
}
