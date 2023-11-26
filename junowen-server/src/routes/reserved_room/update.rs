use anyhow::Result;
use junowen_lib::signaling_server::{
    reserved_room::{
        PostReservedRoomKeepRequestBody, PostReservedRoomKeepResponse,
        PostReservedRoomKeepResponseOkBody, PostReservedRoomKeepResponseOkOpponentAnswerBody,
        PostReservedRoomKeepResponseOkSpectatorAnswerBody, PostReservedRoomSpectateRequestBody,
        PostReservedRoomSpectateResponse,
    },
    room::{PostRoomJoinRequestBody, PostRoomJoinResponse},
};
use tracing::info;
use uuid::Uuid;

use crate::{
    database::{
        Answer, PutError, ReservedRoomOpponentAnswer, ReservedRoomSpectatorAnswer,
        ReservedRoomTables,
    },
    routes::room_utils::{now_sec, ttl_sec, RETRY_AFTER_INTERVAL_SEC},
};

pub async fn find_opponent(
    db: &impl ReservedRoomTables,
    name: String,
) -> Result<Option<ReservedRoomOpponentAnswer>> {
    let Some(answer) = db.remove_room_opponent_answer(name.clone()).await? else {
        return Ok(None);
    };
    db.remove_opponent_offer_sdp_in_room(name).await?;
    Ok(Some(answer))
}

pub async fn find_spectator(
    db: &impl ReservedRoomTables,
    name: String,
) -> Result<Option<ReservedRoomSpectatorAnswer>> {
    let Some(answer) = db.remove_room_spectator_answer(name.clone()).await? else {
        return Ok(None);
    };
    db.remove_spectator_offer_sdp_in_room(name).await?;
    Ok(Some(answer))
}

pub async fn post_room_keep(
    db: &impl ReservedRoomTables,
    name: &str,
    body: PostReservedRoomKeepRequestBody,
) -> Result<PostReservedRoomKeepResponse> {
    let (key, spectator_offer) = body.into_inner();
    if Uuid::parse_str(&key).is_err() {
        return Ok(PostReservedRoomKeepResponse::BadRequest);
    }
    let room = db
        .keep_room(name.to_owned(), key, spectator_offer, ttl_sec(now_sec()))
        .await?;
    let Some(room) = room else {
        return Ok(PostReservedRoomKeepResponse::BadRequest);
    };
    if room.opponent_offer_sdp().is_some() {
        return Ok(
            if let Some(answer) = find_opponent(db, name.to_owned()).await? {
                PostReservedRoomKeepResponseOkBody::from(
                    PostReservedRoomKeepResponseOkOpponentAnswerBody::new(answer.0.into_sdp()),
                )
                .into()
            } else {
                let retry_after = RETRY_AFTER_INTERVAL_SEC;
                PostReservedRoomKeepResponse::NoContent { retry_after }
            },
        );
    }
    if room.spectator_offer_sdp().is_some() {
        return Ok(
            if let Some(answer) = find_spectator(db, name.to_owned()).await? {
                PostReservedRoomKeepResponseOkBody::from(
                    PostReservedRoomKeepResponseOkSpectatorAnswerBody::new(answer.0.into_sdp()),
                )
                .into()
            } else {
                let retry_after = RETRY_AFTER_INTERVAL_SEC;
                PostReservedRoomKeepResponse::NoContent { retry_after }
            },
        );
    }
    let retry_after = RETRY_AFTER_INTERVAL_SEC;
    Ok(PostReservedRoomKeepResponse::NoContent { retry_after })
}

pub async fn post_room_join(
    db: &impl ReservedRoomTables,
    name: &str,
    body: PostRoomJoinRequestBody,
) -> Result<PostRoomJoinResponse> {
    let answer = ReservedRoomOpponentAnswer(Answer::new(
        name.to_owned(),
        body.into_answer(),
        ttl_sec(now_sec()),
    ));
    match db.put_room_opponent_answer(answer).await {
        Ok(()) => {
            info!("[Reserved Room] Join: {}", name);
            Ok(PostRoomJoinResponse::Ok)
        }
        Err(PutError::Conflict) => Ok(PostRoomJoinResponse::Conflict),
        Err(PutError::Unknown(err)) => Err(err),
    }
}

pub async fn post_room_spectate(
    db: &impl ReservedRoomTables,
    name: &str,
    body: PostReservedRoomSpectateRequestBody,
) -> Result<PostReservedRoomSpectateResponse> {
    let answer = ReservedRoomSpectatorAnswer(Answer::new(
        name.to_owned(),
        body.into_answer(),
        ttl_sec(now_sec()),
    ));
    match db.put_room_spectator_answer(answer).await {
        Ok(()) => {
            info!("[Reserved Room] Spectate: {}", name);
            Ok(PostRoomJoinResponse::Ok)
        }
        Err(PutError::Conflict) => Ok(PostRoomJoinResponse::Conflict),
        Err(PutError::Unknown(err)) => Err(err),
    }
}
