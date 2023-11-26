use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};

use crate::connection::signaling::CompressedSdp;

use super::room::{PostRoomKeepResponse, PutRoomResponse, PutRoomResponseAnswerBody};

#[derive(Debug, Deserialize, Serialize, new)]
pub struct PutSharedRoomResponseConflictBody {
    offer: CompressedSdp,
}

impl PutSharedRoomResponseConflictBody {
    pub fn into_offer(self) -> CompressedSdp {
        self.offer
    }
}

pub type PutSharedRoomResponse = PutRoomResponse<PutSharedRoomResponseConflictBody>;

#[derive(Deserialize, Serialize, Getters, new)]
pub struct PostSharedRoomKeepRequestBody {
    key: String,
}

impl PostSharedRoomKeepRequestBody {
    pub fn into_key(self) -> String {
        self.key
    }
}

pub type PostSharedRoomKeepResponse = PostRoomKeepResponse<PutRoomResponseAnswerBody>;
