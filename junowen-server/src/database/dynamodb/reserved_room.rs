use anyhow::{anyhow, Result};
use aws_sdk_dynamodb::{
    error::SdkError,
    types::{AttributeValue, ReturnValue},
};
use junowen_lib::connection::signaling::CompressedSdp;
use serde_dynamo::from_item;

use crate::database::{
    self, PutError, ReservedRoom, ReservedRoomOpponentAnswer, ReservedRoomSpectatorAnswer,
};

use super::DynamoDB;

impl database::ReservedRoomTables for DynamoDB {
    async fn put_room(&self, room: ReservedRoom) -> Result<(), PutError> {
        self.put_item(&self.table_name_reserved_room, room).await
    }

    async fn find_room(&self, name: String) -> Result<Option<ReservedRoom>> {
        self.find_item_by_name(&self.table_name_reserved_room, name)
            .await
    }

    async fn keep_room(
        &self,
        name: String,
        key: String,
        spectator_offer_sdp: Option<CompressedSdp>,
        ttl_sec: u64,
    ) -> Result<Option<ReservedRoom>> {
        let mut builder = self
            .client
            .update_item()
            .table_name(&self.table_name_reserved_room)
            .return_values(ReturnValue::AllNew)
            .key("name", AttributeValue::S(name))
            .expression_attribute_names("#key", "key")
            .expression_attribute_values(":key", AttributeValue::S(key))
            .condition_expression("#key = :key")
            .expression_attribute_names("#ttl_sec", "ttl_sec")
            .expression_attribute_values(":ttl_sec", AttributeValue::N((ttl_sec).to_string()));
        if let Some(spectator_offer_sdp) = spectator_offer_sdp {
            builder = builder
                .expression_attribute_names("#spectator_offer_sdp", "spectator_offer_sdp")
                .expression_attribute_values(
                    ":spectator_offer_sdp",
                    AttributeValue::S(spectator_offer_sdp.into_inner()),
                )
                .update_expression(
                    "SET #ttl_sec = :ttl_sec, #spectator_offer_sdp = :spectator_offer_sdp",
                );
        } else {
            builder = builder.update_expression("SET #ttl_sec = :ttl_sec");
        }
        let result = builder.send().await;
        match result {
            Err(error) => {
                if let SdkError::ServiceError(service_error) = &error {
                    if service_error.err().is_conditional_check_failed_exception() {
                        return Ok(None);
                    }
                }
                Err(error.into())
            }
            Ok(output) => {
                let item = output
                    .attributes()
                    .ok_or_else(|| anyhow!("attributes not found"))?;
                Ok(Some(from_item(item.to_owned())?))
            }
        }
    }

    async fn remove_opponent_offer_sdp_in_room(&self, name: String) -> Result<bool> {
        let result = self
            .client
            .update_item()
            .table_name(&self.table_name_reserved_room)
            .key("name", AttributeValue::S(name))
            .update_expression("SET #opponent_offer_sdp = :opponent_offer_sdp")
            .expression_attribute_names("#opponent_offer_sdp", "opponent_offer_sdp")
            .expression_attribute_values(":opponent_offer_sdp", AttributeValue::Null(true))
            .send()
            .await;
        if let Err(err) = result {
            if let SdkError::ServiceError(service_error) = &err {
                if service_error.err().is_conditional_check_failed_exception() {
                    return Ok(false);
                }
            }
            return Err(err.into());
        }
        Ok(true)
    }

    async fn remove_spectator_offer_sdp_in_room(&self, name: String) -> Result<bool> {
        let result = self
            .client
            .update_item()
            .table_name(&self.table_name_reserved_room)
            .key("name", AttributeValue::S(name))
            .update_expression("SET #spectator_offer_sdp = :spectator_offer_sdp")
            .expression_attribute_names("#spectator_offer_sdp", "spectator_offer_sdp")
            .expression_attribute_values(":spectator_offer_sdp", AttributeValue::Null(true))
            .send()
            .await;
        if let Err(err) = result {
            if let SdkError::ServiceError(service_error) = &err {
                if service_error.err().is_conditional_check_failed_exception() {
                    return Ok(false);
                }
            }
            return Err(err.into());
        }
        Ok(true)
    }

    async fn remove_room(&self, name: String, key: Option<String>) -> Result<bool> {
        self.remove_item(&self.table_name_reserved_room, name, key)
            .await
    }

    async fn put_room_opponent_answer(
        &self,
        answer: ReservedRoomOpponentAnswer,
    ) -> Result<(), PutError> {
        self.put_item(&self.table_name_reserved_room_opponent_answer, answer)
            .await
    }

    async fn remove_room_opponent_answer(
        &self,
        name: String,
    ) -> Result<Option<ReservedRoomOpponentAnswer>> {
        self.remove_item_and_get_old(&self.table_name_reserved_room_opponent_answer, name)
            .await
    }

    async fn put_room_spectator_answer(
        &self,
        answer: ReservedRoomSpectatorAnswer,
    ) -> Result<(), PutError> {
        self.put_item(&self.table_name_reserved_room_spectator_answer, answer)
            .await
    }

    async fn remove_room_spectator_answer(
        &self,
        name: String,
    ) -> Result<Option<ReservedRoomSpectatorAnswer>> {
        self.remove_item_and_get_old(&self.table_name_reserved_room_spectator_answer, name)
            .await
    }
}
