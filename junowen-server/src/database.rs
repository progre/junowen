mod dynamodb;
mod file;

use derive_new::new;
pub use dynamodb::DynamoDB;
pub use file::File;

use anyhow::Result;
use async_trait::async_trait;
use getset::Getters;
use junowen_lib::connection::signaling::CompressedSdp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Getters, Serialize, new)]
pub struct SharedRoomOffer {
    /// primary
    #[get = "pub"]
    name: String,
    /// ルームの所有者であることを証明する為のキー
    #[get = "pub"]
    key: String,
    #[get = "pub"]
    sdp: CompressedSdp,
    ttl_sec: u64,
}

impl SharedRoomOffer {
    pub fn into_sdp(self) -> CompressedSdp {
        self.sdp
    }

    pub fn is_expired(&self, now_sec: u64) -> bool {
        now_sec > self.ttl_sec
    }
}

#[derive(Serialize, Getters, Deserialize, new)]
pub struct Answer {
    /// primary
    #[get = "pub"]
    name: String,
    #[get = "pub"]
    sdp: CompressedSdp,
    ttl_sec: u64,
}

impl Answer {
    pub fn into_sdp(self) -> CompressedSdp {
        self.sdp
    }
}

#[derive(Deserialize, Serialize)]
pub struct SharedRoomAnswer(pub Answer);

#[derive(Debug)]
pub enum PutError {
    Conflict,
    Unknown(anyhow::Error),
}

#[async_trait]
pub trait Database: Send + Sync + 'static {
    async fn put_shared_room_offer(&self, offer: SharedRoomOffer) -> Result<(), PutError>;
    async fn find_shared_room_offer(&self, name: String) -> Result<Option<SharedRoomOffer>>;
    async fn keep_shared_room_offer(
        &self,
        name: String,
        key: String,
        ttl_sec: u64,
    ) -> Result<Option<()>>;
    async fn remove_shared_room_offer(&self, name: String) -> Result<()>;
    async fn remove_shared_room_offer_with_key(&self, name: String, key: String) -> Result<bool>;
    async fn put_shared_room_answer(&self, answer: SharedRoomAnswer) -> Result<(), PutError>;
    async fn remove_shared_room_answer(&self, name: String) -> Result<Option<SharedRoomAnswer>>;
}
