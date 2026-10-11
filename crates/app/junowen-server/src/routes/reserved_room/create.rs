use anyhow::{bail, Result};
use junowen_lib::signaling_server::{
    reserved_room::{PutReservedRoomResponse, PutReservedRoomResponseConflictBody},
    room::{PutRoomRequestBody, PutRoomResponseAnswerBody, PutRoomResponseWaitingBody},
};
use tracing::info;
use uuid::Uuid;

use crate::{
    database::{PutError, ReservedRoom, ReservedRoomTables},
    routes::{
        reserved_room::{read::find_valid_room, update::find_opponent},
        room_utils::{now_sec, ttl_sec, RETRY_AFTER_INTERVAL_SEC},
    },
};

pub async fn put_room(
    db: &impl ReservedRoomTables,
    name: &str,
    body: PutRoomRequestBody,
) -> Result<PutReservedRoomResponse> {
    let now_sec = now_sec();
    let key = Uuid::new_v4().to_string();
    let room = ReservedRoom::new(
        name.to_owned(),
        key.clone(),
        Some(body.offer().clone()),
        None,
        ttl_sec(now_sec),
    );
    for retry in 0.. {
        if let Some(room) = find_valid_room(db, now_sec, name.to_owned()).await? {
            let body = PutReservedRoomResponseConflictBody::new(room.into_opponent_offer_sdp());
            let response = PutReservedRoomResponse::conflict(RETRY_AFTER_INTERVAL_SEC, body);
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
    info!("[Reserved Room] Created: {}", name);
    Ok(
        if let Some(answer) = find_opponent(db, name.to_owned()).await? {
            let body = PutRoomResponseAnswerBody::new(answer.0.into_sdp());
            PutReservedRoomResponse::created_with_answer(RETRY_AFTER_INTERVAL_SEC, body)
        } else {
            let body = PutRoomResponseWaitingBody::new(key);
            PutReservedRoomResponse::created_with_key(RETRY_AFTER_INTERVAL_SEC, body)
        },
    )
}
