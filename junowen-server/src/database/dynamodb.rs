mod shared_room;

use std::env;

use anyhow::Result;
use aws_sdk_dynamodb::{
    error::SdkError,
    types::{AttributeValue, ReturnValue},
};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, to_item};

use super::{Database, PutError};

pub struct DynamoDB {
    client: aws_sdk_dynamodb::Client,
    table_name_shared_room: String,
    table_name_shared_room_opponent_answer: String,
}

impl DynamoDB {
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;
        Self {
            client: aws_sdk_dynamodb::Client::new(&config),
            table_name_shared_room: format!("{}.Offer", env::var("ENV").unwrap()),
            table_name_shared_room_opponent_answer: format!("{}.Answer", env::var("ENV").unwrap()),
        }
    }

    pub async fn put_item(&self, table_name: &str, item: impl Serialize) -> Result<(), PutError> {
        let item = to_item(item).map_err(|err| PutError::Unknown(err.into()))?;
        let result = self
            .client
            .put_item()
            .table_name(table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(#name)")
            .expression_attribute_names("#name", "name")
            .send()
            .await;
        if let Err(err) = result {
            if let SdkError::ServiceError(service_error) = &err {
                if service_error.err().is_conditional_check_failed_exception() {
                    return Err(PutError::Conflict);
                }
            }
            return Err(PutError::Unknown(err.into()));
        }
        Ok(())
    }

    async fn find_item_by_name<'a, T>(&self, table_name: &str, name: String) -> Result<Option<T>>
    where
        T: Deserialize<'a>,
    {
        let output = self
            .client
            .query()
            .table_name(table_name)
            .key_condition_expression("#name = :name")
            .expression_attribute_names("#name", "name")
            .expression_attribute_values(":name", AttributeValue::S(name))
            .send()
            .await?;
        let Some(items) = output.items() else {
            return Ok(None);
        };
        let Some(item) = items.first() else {
            return Ok(None);
        };
        Ok(Some(from_item(item.to_owned())?))
    }

    async fn remove_item_and_get_old<'a, T>(
        &self,
        table_name: &str,
        name: String,
    ) -> Result<Option<T>>
    where
        T: Deserialize<'a>,
    {
        let output = self
            .client
            .delete_item()
            .table_name(table_name)
            .key("name", AttributeValue::S(name))
            .return_values(ReturnValue::AllOld)
            .send()
            .await?;
        let Some(item) = output.attributes() else {
            return Ok(None);
        };
        Ok(Some(from_item(item.to_owned())?))
    }

    async fn remove_item(
        &self,
        table_name: &str,
        name: String,
        key: Option<String>,
    ) -> Result<bool> {
        let mut builder = self
            .client
            .delete_item()
            .table_name(table_name)
            .key("name", AttributeValue::S(name));
        if let Some(key) = key {
            builder = builder
                .condition_expression("#key = :key")
                .expression_attribute_names("#key", "key")
                .expression_attribute_values(":key", AttributeValue::S(key));
        }
        let result = builder.send().await;
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
}

impl Database for DynamoDB {}
