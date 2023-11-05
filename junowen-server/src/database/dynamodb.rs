use std::{collections::HashMap, env};

use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_dynamodb::{error::SdkError, types::AttributeValue, Client};
use serde_dynamo::{from_item, to_item};

use super::{Answer, Database, Offer, PutError};

async fn put_item(
    client: &Client,
    table_name: &str,
    item: HashMap<String, AttributeValue>,
) -> Result<(), PutError> {
    if let Err(error) = client
        .put_item()
        .table_name(table_name)
        .set_item(Some(item))
        .condition_expression("attribute_not_exists(#name)")
        .expression_attribute_names("#name", "name")
        .send()
        .await
    {
        let SdkError::ServiceError(err) = &error else {
            return Err(PutError::Unknown(error.into()));
        };
        if !err.err().is_conditional_check_failed_exception() {
            return Err(PutError::Unknown(error.into()));
        }
        return Err(PutError::Conflict);
    }
    Ok(())
}

pub struct DynamoDB {
    client: aws_sdk_dynamodb::Client,
    table_name_offer: String,
    table_name_answer: String,
}

impl DynamoDB {
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;
        Self {
            client: aws_sdk_dynamodb::Client::new(&config),
            table_name_offer: format!("{}.Offer", env::var("ENV").unwrap()),
            table_name_answer: format!("{}.Answer", env::var("ENV").unwrap()),
        }
    }
}

#[async_trait]
impl Database for DynamoDB {
    async fn find_offer(&self, name: String) -> Result<Option<Offer>> {
        let output = self
            .client
            .query()
            .table_name(&self.table_name_offer)
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
        Ok(Some(from_item::<_, Offer>(item.to_owned())?))
    }

    async fn put_offer(&self, offer: Offer) -> Result<(), PutError> {
        let item = to_item(offer).map_err(|err| PutError::Unknown(err.into()))?;
        put_item(&self.client, &self.table_name_offer, item).await
    }

    async fn keep_offer(&self, name: String, key: String, ttl_sec: u64) -> Result<Option<()>> {
        let result = self
            .client
            .update_item()
            .table_name(&self.table_name_offer)
            .key("name", AttributeValue::S(name))
            .condition_expression("#key = :key")
            .update_expression("SET #ttl_sec = :ttl_sec")
            .expression_attribute_names("#key", "key")
            .expression_attribute_names("#ttl_sec", "ttl_sec")
            .expression_attribute_values(":key", AttributeValue::S(key))
            .expression_attribute_values(":ttl_sec", AttributeValue::N((ttl_sec).to_string()))
            .send()
            .await;
        match result {
            Ok(_) => return Ok(Some(())),
            Err(error) => {
                let SdkError::ServiceError(err) = &error else {
                    return Err(error.into());
                };
                if !err.err().is_conditional_check_failed_exception() {
                    return Err(error.into());
                }
                return Ok(None);
            }
        }
    }

    async fn find_answer(&self, name: String) -> Result<Option<Answer>> {
        let output = self
            .client
            .query()
            .table_name(&self.table_name_answer)
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

    async fn put_answer(&self, answer: Answer) -> Result<(), PutError> {
        let item = to_item(answer).map_err(|err| PutError::Unknown(err.into()))?;
        put_item(&self.client, &self.table_name_answer, item).await
    }

    async fn remove_offer(&self, name: String) -> Result<()> {
        let _output = self
            .client
            .delete_item()
            .table_name(&self.table_name_offer)
            .key("name", AttributeValue::S(name))
            .send()
            .await?;
        Ok(())
    }

    async fn remove_offer_with_key(&self, name: String, key: String) -> Result<bool> {
        if let Err(err) = self
            .client
            .delete_item()
            .table_name(&self.table_name_offer)
            .key("name", AttributeValue::S(name))
            .condition_expression("#key = :key")
            .expression_attribute_names("#key", "key")
            .expression_attribute_values(":key", AttributeValue::S(key))
            .send()
            .await
        {
            if let SdkError::ServiceError(service_error) = &err {
                if service_error.err().is_resource_not_found_exception() {
                    return Ok(false);
                }
            }
            return Err(err.into());
        }
        Ok(true)
    }

    async fn remove_answer(&self, name: String) -> Result<()> {
        let _output = self
            .client
            .delete_item()
            .table_name(&self.table_name_answer)
            .key("name", AttributeValue::S(name))
            .send()
            .await?;
        Ok(())
    }
}
