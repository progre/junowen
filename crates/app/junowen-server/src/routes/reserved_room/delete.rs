use anyhow::Result;
use junowen_lib::signaling_server::room::{DeleteRoomRequestBody, DeleteRoomResponse};
use tracing::info;

use crate::database::ReservedRoomTables;

pub async fn delete_room(
    db: &impl ReservedRoomTables,
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
        info!("[Reserved Room] Removed: {}", name);
        Ok(DeleteRoomResponse::NoContent)
    }
}
