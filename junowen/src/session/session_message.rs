use junowen_lib::structs::settings::GameSettings;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchInitial {
    pub game_settings: GameSettings,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RoundInitial {
    pub seed1: u16,
    pub seed2: u16,
    pub seed3: u16,
    pub seed4: u16,
}

/** input 以外はホストのみ発行できる */
#[derive(Debug, Deserialize, Serialize)]
pub enum SessionMessage {
    InitMatch((String, Option<MatchInitial>)),
    InitRound(Option<RoundInitial>),
    Delay(u8),
    Input(u16),
}
