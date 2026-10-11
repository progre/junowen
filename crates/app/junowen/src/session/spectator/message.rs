use derive_new::new;
use getset::{CopyGetters, Getters};
use junowen_lib::structs::settings::GameSettings;
use serde::{Deserialize, Serialize};

use crate::session::RoundInitial;

#[derive(new, Clone, Debug, Deserialize, Getters, Serialize)]
pub struct SpectatorInitial {
    #[get = "pub"]
    p1_name: String,
    #[get = "pub"]
    p2_name: String,
    #[get = "pub"]
    game_settings: GameSettings,
}

#[derive(new, Clone, Copy, Debug, Deserialize, CopyGetters, PartialEq, Serialize)]
pub struct PlayerGameInitial {
    #[get_copy = "pub"]
    character: u8,
    #[get_copy = "pub"]
    card: u8,
}

/// 試合開始時の状態。観戦者はこれに合わせてキャラクターとカードを決定する
#[derive(new, Clone, Debug, Deserialize, CopyGetters, Getters, Serialize)]
pub struct GameInitial {
    #[get_copy = "pub"]
    difficulty: u8,
    #[get_copy = "pub"]
    p1: PlayerGameInitial,
    #[get_copy = "pub"]
    p2: PlayerGameInitial,
    #[get = "pub"]
    round_initial: RoundInitial,
}

/// 観戦ホストが観戦者を受け付ける専有ルームの名前とキーを表す。
#[derive(new, Clone, Debug, Deserialize, Getters, Serialize)]
pub struct SpectatorRelayRoom {
    #[get = "pub"]
    room_name: String,
    #[get = "pub"]
    key: String,
}

/// 観戦ホストへの任命
#[derive(new, Clone, Debug, Deserialize, Serialize)]
pub struct SpectatorHostDelegation {
    /// 観戦者を待ち受ける専有ルームの情報を持つ。ない場合は、Pure P2P で観戦者を待ち受ける。
    room: Option<SpectatorRelayRoom>,
}

impl SpectatorHostDelegation {
    pub fn into_room(self) -> Option<SpectatorRelayRoom> {
        self.room
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SpectatorSessionMessage {
    InitSpectator(SpectatorInitial),
    InitGame(GameInitial),
    InitRound(RoundInitial),
    Inputs(u16, u16),
    /// 受信した観戦者を観戦ホストに任命し、以降の観戦者の受け付けと中継を委譲する
    DelegateSpectatorHost(SpectatorHostDelegation),
}
