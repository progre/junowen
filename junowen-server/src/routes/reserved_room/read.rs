use anyhow::Result;
use junowen_lib::signaling_server::reserved_room::{
    GetReservedRoomResponse, GetReservedRoomResponseOkBody,
};

use crate::{
    database::{ReservedRoom, ReservedRoomTables},
    routes::room_utils::now_sec,
};

pub async fn find_valid_room(
    db: &impl ReservedRoomTables,
    now_sec: u64,
    name: String,
) -> Result<Option<ReservedRoom>> {
    let Some(offer) = db.find_room(name.to_owned()).await? else {
        return Ok(None);
    };
    if !offer.is_expired(now_sec) {
        return Ok(Some(offer));
    }
    db.remove_room(offer.name().clone(), None).await?;
    db.remove_room_opponent_answer(offer.name().clone()).await?;
    db.remove_room_spectator_answer(offer.name().clone())
        .await?;
    Ok(None)
}

pub async fn get_room(db: &impl ReservedRoomTables, name: &str) -> Result<GetReservedRoomResponse> {
    let now_sec = now_sec();
    let Some(room) = find_valid_room(db, now_sec, name.to_owned()).await? else {
        return Ok(GetReservedRoomResponse::NotFound);
    };
    let (opponent_offer_sdp, spectator_offer_sdp) =
        room.into_opponent_offer_sdp_spectator_offer_sdp();
    let body = GetReservedRoomResponseOkBody::new(opponent_offer_sdp, spectator_offer_sdp);
    Ok(GetReservedRoomResponse::Ok(body))
}
