mod dynamodb;
mod file;

use derive_new::new;
pub use dynamodb::DynamoDB;
pub use file::File;

use anyhow::Result;
use getset::{Getters, Setters};
use junowen_lib::connection::signaling::CompressedSdp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Getters, Serialize, new)]
pub struct SharedRoom {
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

impl SharedRoom {
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

pub type SharedRoomOpponentAnswer = Answer;

#[derive(Debug)]
pub enum PutError {
    Conflict,
    Unknown(anyhow::Error),
}

pub trait SharedRoomTables: Send + Sync + 'static {
    async fn put_room(&self, offer: SharedRoom) -> Result<(), PutError>;
    async fn find_room(&self, name: String) -> Result<Option<SharedRoom>>;
    async fn keep_room(&self, name: String, key: String, ttl_sec: u64) -> Result<bool>;
    async fn remove_room(&self, name: String, key: Option<String>) -> Result<bool>;

    async fn put_room_opponent_answer(
        &self,
        answer: SharedRoomOpponentAnswer,
    ) -> Result<(), PutError>;
    async fn remove_room_opponent_answer(
        &self,
        name: String,
    ) -> Result<Option<SharedRoomOpponentAnswer>>;
}

#[derive(Clone, Debug, Deserialize, Getters, Setters, Serialize, new)]
pub struct ReservedRoom {
    /// primary
    #[get = "pub"]
    name: String,
    /// ルームの所有者であることを証明する為のキー
    #[get = "pub"]
    key: String,
    #[getset(get = "pub", set = "pub")]
    opponent_offer_sdp: Option<CompressedSdp>,
    #[get = "pub"]
    spectator_offer_sdp: Option<CompressedSdp>,
    ttl_sec: u64,
}

impl ReservedRoom {
    pub fn into_opponent_offer_sdp(self) -> Option<CompressedSdp> {
        self.opponent_offer_sdp
    }

    pub fn into_opponent_offer_sdp_spectator_offer_sdp(
        self,
    ) -> (Option<CompressedSdp>, Option<CompressedSdp>) {
        (self.opponent_offer_sdp, self.spectator_offer_sdp)
    }

    pub fn is_expired(&self, now_sec: u64) -> bool {
        now_sec > self.ttl_sec
    }
}

#[derive(Serialize, Deserialize)]
pub struct ReservedRoomOpponentAnswer(pub Answer);
#[derive(Serialize, Deserialize)]
pub struct ReservedRoomSpectatorAnswer(pub Answer);

pub trait ReservedRoomTables: Send + Sync + 'static {
    async fn put_room(&self, offer: ReservedRoom) -> Result<(), PutError>;
    async fn find_room(&self, name: String) -> Result<Option<ReservedRoom>>;
    async fn keep_room(
        &self,
        name: String,
        key: String,
        spectator_offer_sdp: Option<CompressedSdp>,
        ttl_sec: u64,
    ) -> Result<Option<ReservedRoom>>;
    async fn remove_opponent_offer_sdp_in_room(&self, name: String) -> Result<bool>;
    async fn remove_spectator_offer_sdp_in_room(&self, name: String) -> Result<bool>;
    async fn remove_room(&self, name: String, key: Option<String>) -> Result<bool>;

    async fn put_room_opponent_answer(
        &self,
        answer: ReservedRoomOpponentAnswer,
    ) -> Result<(), PutError>;
    async fn remove_room_opponent_answer(
        &self,
        name: String,
    ) -> Result<Option<ReservedRoomOpponentAnswer>>;

    async fn put_room_spectator_answer(
        &self,
        answer: ReservedRoomSpectatorAnswer,
    ) -> Result<(), PutError>;
    async fn remove_room_spectator_answer(
        &self,
        name: String,
    ) -> Result<Option<ReservedRoomSpectatorAnswer>>;
}

pub trait Database: SharedRoomTables + ReservedRoomTables {}
