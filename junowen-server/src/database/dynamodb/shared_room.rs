use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_dynamodb::{error::SdkError, types::AttributeValue};

use crate::database::{PutError, SharedRoom, SharedRoomOpponentAnswer, SharedRoomTables};

use super::DynamoDB;

#[async_trait]
impl SharedRoomTables for DynamoDB {
    async fn put_room(&self, room: SharedRoom) -> Result<(), PutError> {
        self.put_item(&self.table_name_shared_room, room).await
    }

    async fn find_room(&self, name: String) -> Result<Option<SharedRoom>> {
        self.find_item_by_name(&self.table_name_shared_room, name)
            .await
    }

    async fn keep_room(&self, name: String, key: String, ttl_sec: u64) -> Result<bool> {
        let result = self
            .client
            .update_item()
            .table_name(&self.table_name_shared_room)
            .key("name", AttributeValue::S(name))
            .condition_expression("#key = :key")
            .update_expression("SET #ttl_sec = :ttl_sec")
            .expression_attribute_names("#key", "key")
            .expression_attribute_values(":key", AttributeValue::S(key))
            .expression_attribute_names("#ttl_sec", "ttl_sec")
            .expression_attribute_values(":ttl_sec", AttributeValue::N((ttl_sec).to_string()))
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
        self.remove_item(&self.table_name_shared_room, name, key)
            .await
    }

    async fn put_room_opponent_answer(
        &self,
        answer: SharedRoomOpponentAnswer,
    ) -> Result<(), PutError> {
        self.put_item(&self.table_name_shared_room_opponent_answer, answer)
            .await
    }

    async fn remove_room_opponent_answer(
        &self,
        name: String,
    ) -> Result<Option<SharedRoomOpponentAnswer>> {
        self.remove_item_and_get_old(&self.table_name_shared_room_opponent_answer, name)
            .await
    }
}
