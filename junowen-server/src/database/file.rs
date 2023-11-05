use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs;

use super::{Answer, Database, Offer, PutError};

pub struct File;

impl File {
    async fn read(&self) -> Result<Value> {
        Ok(
            serde_json::from_str(&fs::read_to_string("store.json").await.unwrap_or_default())
                .unwrap_or_default(),
        )
    }

    async fn write(&self, value: Value) -> Result<()> {
        Ok(fs::write("store.json", serde_json::to_string_pretty(&value)?).await?)
    }
}

#[async_trait]
impl Database for File {
    async fn find_offer(&self, name: String) -> Result<Option<Offer>> {
        let store = self.read().await?;
        let Some(offers) = store.get("offers") else {
            return Ok(None);
        };
        let Some(offers) = offers.as_array() else {
            return Ok(None);
        };
        Ok(offers
            .iter()
            .map(|x| serde_json::from_value(x.clone()).unwrap())
            .find(|x: &Offer| x.name == name))
    }

    async fn put_offer(&self, offer: Offer) -> Result<(), PutError> {
        let mut store = self.read().await.map_err(PutError::Unknown)?;
        if store.get("offers").is_none() {
            store["offers"] = Value::Array(vec![]);
        }
        let array = store["offers"].as_array_mut().unwrap();
        if array
            .iter()
            .map(|x| serde_json::from_value::<Offer>(x.clone()).unwrap())
            .any(|x| x.name() == &offer.name)
        {
            return Err(PutError::Conflict);
        }
        array.push(serde_json::to_value(offer).map_err(|err| PutError::Unknown(err.into()))?);

        self.write(store).await.map_err(PutError::Unknown)?;
        Ok(())
    }

    async fn keep_offer(&self, name: String, key: String, ttl_sec: u64) -> Result<Option<()>> {
        let mut store = self.read().await?;
        if store.get("offers").is_none() {
            store["offers"] = Value::Array(vec![]);
        }
        let array = store["offers"].as_array_mut().unwrap();
        let Some(offer) = array.iter_mut().find(|x| {
            let offer = serde_json::from_value::<Offer>((*x).to_owned()).unwrap();
            offer.name() == &name && offer.key() == &key
        }) else {
            return Ok(None);
        };
        let mut new_offer = serde_json::from_value::<Offer>(offer.to_owned()).unwrap();
        new_offer.ttl_sec = ttl_sec;
        *offer = serde_json::to_value(new_offer).unwrap();
        self.write(store).await?;
        Ok(Some(()))
    }

    async fn find_answer(&self, name: String) -> Result<Option<Answer>> {
        let store = self.read().await?;
        let Some(offers) = store.get("answer") else {
            return Ok(None);
        };
        let Some(offers) = offers.as_array() else {
            return Ok(None);
        };
        Ok(offers
            .iter()
            .filter_map(|x| serde_json::from_value(x.clone()).unwrap())
            .find(|x: &Answer| x.name == name))
    }

    async fn put_answer(&self, answer: Answer) -> Result<(), PutError> {
        let mut store = self.read().await.map_err(PutError::Unknown)?;
        if store.get("answers").is_none() {
            store["answers"] = Value::Array(vec![]);
        }
        let array = store["answers"].as_array_mut().unwrap();
        if array
            .iter()
            .map(|x| serde_json::from_value::<Offer>(x.clone()).unwrap())
            .any(|x| x.name() == &answer.name)
        {
            return Err(PutError::Conflict);
        }
        array.push(serde_json::to_value(answer).map_err(|err| PutError::Unknown(err.into()))?);

        self.write(store).await.map_err(PutError::Unknown)?;
        Ok(())
    }

    async fn remove_offer(&self, name: String) -> Result<()> {
        let mut store = self.read().await?;
        if store.get("offers").is_none() {
            store["offers"] = Value::Array(vec![]);
        }
        store["offers"]
            .as_array_mut()
            .unwrap()
            .retain(|x| serde_json::from_value::<Offer>(x.clone()).unwrap().name == name);
        self.write(store).await?;
        Ok(())
    }

    async fn remove_offer_with_key(&self, _name: String, _key: String) -> Result<bool> {
        unimplemented!()
    }

    async fn remove_answer(&self, _name: String) -> Result<()> {
        unimplemented!()
    }
}
