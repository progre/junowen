use anyhow::Result;
use serde_json::Value;
use tokio::fs;

use super::{
    Answer, Database, PutError, ReservedRoom, ReservedRoomOpponentAnswer,
    ReservedRoomSpectatorAnswer, ReservedRoomTables, SharedRoom, SharedRoomOpponentAnswer,
    SharedRoomTables,
};

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

impl SharedRoomTables for File {
    async fn put_room(&self, offer: SharedRoom) -> Result<(), PutError> {
        let mut store = self.read().await.map_err(PutError::Unknown)?;
        if store.get("offers").is_none() {
            store["offers"] = Value::Array(vec![]);
        }
        let array = store["offers"].as_array_mut().unwrap();
        if array
            .iter()
            .map(|x| serde_json::from_value::<SharedRoom>(x.clone()).unwrap())
            .any(|x| x.name() == &offer.name)
        {
            return Err(PutError::Conflict);
        }
        array.push(serde_json::to_value(offer).map_err(|err| PutError::Unknown(err.into()))?);

        self.write(store).await.map_err(PutError::Unknown)?;
        Ok(())
    }

    async fn find_room(&self, name: String) -> Result<Option<SharedRoom>> {
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
            .find(|x: &SharedRoom| x.name == name))
    }

    async fn keep_room(&self, name: String, key: String, ttl_sec: u64) -> Result<bool> {
        let mut store = self.read().await?;
        if store.get("offers").is_none() {
            store["offers"] = Value::Array(vec![]);
        }
        let array = store["offers"].as_array_mut().unwrap();
        let Some(offer) = array.iter_mut().find(|x| {
            let offer = serde_json::from_value::<SharedRoom>((*x).to_owned()).unwrap();
            offer.name() == &name && offer.key() == &key
        }) else {
            return Ok(false);
        };
        let mut new_offer = serde_json::from_value::<SharedRoom>(offer.to_owned()).unwrap();
        new_offer.ttl_sec = ttl_sec;
        *offer = serde_json::to_value(new_offer).unwrap();
        self.write(store).await?;
        unimplemented!()
    }

    async fn remove_room(&self, name: String, _key: Option<String>) -> Result<bool> {
        let mut store = self.read().await?;
        if store.get("offers").is_none() {
            store["offers"] = Value::Array(vec![]);
        }
        store["offers"].as_array_mut().unwrap().retain(|x| {
            serde_json::from_value::<SharedRoom>(x.clone())
                .unwrap()
                .name
                == name
        });
        self.write(store).await?;
        todo!()
    }

    async fn put_room_opponent_answer(
        &self,
        answer: SharedRoomOpponentAnswer,
    ) -> Result<(), PutError> {
        let mut store = self.read().await.map_err(PutError::Unknown)?;
        if store.get("answers").is_none() {
            store["answers"] = Value::Array(vec![]);
        }
        let array = store["answers"].as_array_mut().unwrap();
        if array
            .iter()
            .map(|x| serde_json::from_value::<SharedRoom>(x.clone()).unwrap())
            .any(|x| x.name() == &answer.name)
        {
            return Err(PutError::Conflict);
        }
        array.push(serde_json::to_value(answer).map_err(|err| PutError::Unknown(err.into()))?);

        self.write(store).await.map_err(PutError::Unknown)?;
        Ok(())
    }

    async fn remove_room_opponent_answer(
        &self,
        name: String,
    ) -> Result<Option<SharedRoomOpponentAnswer>> {
        let store = self.read().await?;
        let Some(offers) = store.get("answer") else {
            return Ok(None);
        };
        let Some(offers) = offers.as_array() else {
            return Ok(None);
        };
        let _item = offers
            .iter()
            .filter_map(|x| serde_json::from_value(x.clone()).unwrap())
            .find(|x: &Answer| x.name == name);

        unimplemented!()
    }
}

impl ReservedRoomTables for File {
    async fn put_room(&self, _offer: ReservedRoom) -> Result<(), PutError> {
        unimplemented!();
    }

    async fn find_room(&self, _name: String) -> Result<Option<ReservedRoom>> {
        unimplemented!();
    }

    async fn remove_room(&self, _name: String, _key: Option<String>) -> Result<bool> {
        unimplemented!();
    }

    async fn remove_opponent_offer_sdp_in_room(&self, _name: String) -> Result<bool> {
        unimplemented!()
    }

    async fn remove_spectator_offer_sdp_in_room(&self, _name: String) -> Result<bool> {
        unimplemented!()
    }

    async fn put_room_opponent_answer(
        &self,
        _answer: ReservedRoomOpponentAnswer,
    ) -> Result<(), PutError> {
        unimplemented!()
    }

    async fn remove_room_opponent_answer(
        &self,
        _name: String,
    ) -> Result<Option<ReservedRoomOpponentAnswer>> {
        unimplemented!()
    }

    async fn put_room_spectator_answer(
        &self,
        _answer: ReservedRoomSpectatorAnswer,
    ) -> Result<(), PutError> {
        unimplemented!()
    }

    async fn remove_room_spectator_answer(
        &self,
        _name: String,
    ) -> Result<Option<ReservedRoomSpectatorAnswer>> {
        unimplemented!()
    }

    async fn keep_room(
        &self,
        _name: String,
        _key: String,
        _spectator_offer_sdp: Option<String>,
        _ttl_sec: u64,
    ) -> Result<Option<ReservedRoom>> {
        unimplemented!()
    }
}

impl Database for File {}
