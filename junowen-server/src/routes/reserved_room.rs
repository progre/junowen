mod create;
mod delete;
mod read;
mod update;

use anyhow::Result;
use lambda_http::{
    http::{Method, StatusCode},
    Body, Request, Response,
};
use regex::Regex;
use tracing::debug;

use crate::database::ReservedRoomTables;

use self::{
    create::put_room,
    delete::delete_room,
    read::get_room,
    update::{post_room_join, post_room_keep, post_room_spectate},
};

use super::{
    room_utils::{decode_room_name, from_post_room_keep_response, from_put_room_response},
    to_response, try_parse,
};

pub async fn route(
    relative_uri: &str,
    req: &Request,
    db: &impl ReservedRoomTables,
) -> Result<Response<Body>> {
    let regex = Regex::new(r"^([^/]+)$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        let room_name = match decode_room_name(&c[1]) {
            Ok(room_name) => room_name,
            Err(err) => {
                debug!("{:?}", err);
                return Ok(to_response(StatusCode::NOT_FOUND, Body::Empty));
            }
        };
        return Ok(match *req.method() {
            Method::PUT => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = put_room(db, &room_name, body).await?;
                    from_put_room_response(res)
                }
            },
            Method::GET => {
                let res = get_room(db, &room_name).await?;
                to_response(
                    res.status_code(),
                    res.to_body().map(Body::Text).unwrap_or_else(|| Body::Empty),
                )
            }
            Method::DELETE => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = delete_room(db, &room_name, body).await?;
                    to_response(res.status_code(), Body::Empty)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, Body::Empty),
        });
    }
    let regex = Regex::new(r"^([^/]+)/join$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        let room_name = match decode_room_name(&c[1]) {
            Ok(room_name) => room_name,
            Err(err) => {
                debug!("{:?}", err);
                return Ok(to_response(StatusCode::NOT_FOUND, Body::Empty));
            }
        };
        return Ok(match *req.method() {
            Method::POST => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = post_room_join(db, &room_name, body).await?;
                    to_response(res.status_code_old(), Body::Empty)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, Body::Empty),
        });
    }
    let regex = Regex::new(r"^([^/]+)/spectate$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        let room_name = match decode_room_name(&c[1]) {
            Ok(room_name) => room_name,
            Err(err) => {
                debug!("{:?}", err);
                return Ok(to_response(StatusCode::NOT_FOUND, Body::Empty));
            }
        };
        return Ok(match *req.method() {
            Method::POST => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = post_room_spectate(db, &room_name, body).await?;
                    to_response(res.status_code_old(), Body::Empty)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, Body::Empty),
        });
    }
    let regex = Regex::new(r"^([^/]+)/keep$").unwrap();
    if let Some(c) = regex.captures(relative_uri) {
        let room_name = match decode_room_name(&c[1]) {
            Ok(room_name) => room_name,
            Err(err) => {
                debug!("{:?}", err);
                return Ok(to_response(StatusCode::NOT_FOUND, Body::Empty));
            }
        };
        return Ok(match *req.method() {
            Method::POST => match try_parse(req.body()) {
                Err(err) => {
                    debug!("{:?}", err);
                    to_response(StatusCode::BAD_REQUEST, Body::Empty)
                }
                Ok(body) => {
                    let res = post_room_keep(db, &room_name, body).await?;
                    from_post_room_keep_response(res)
                }
            },
            _ => to_response(StatusCode::METHOD_NOT_ALLOWED, Body::Empty),
        });
    }
    Ok(to_response(StatusCode::NOT_FOUND, Body::Empty))
}
