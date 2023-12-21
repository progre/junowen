use derive_new::new;
use getset::{CopyGetters, Setters};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, TryFromPrimitive)]
#[repr(u8)]
pub enum TimeLimit {
    #[default]
    None = 0,
    ThreeMinutes = 1,
    FiveMinutes = 2,
    SevenMinutes = 3,
    TenMinutes = 4,
    FifteenMinutes = 5,
    TwentyMinutes = 6,
}

#[derive(Clone, Copy, Default, TryFromPrimitive)]
#[repr(u8)]
pub enum Round {
    /// 1本勝負
    #[default]
    SingleMatch = 0,
    /// 2本先取
    TwoOutOfThree = 1,
    /// 3本先取
    ThreeOutOfFive = 2,
}

#[derive(Clone, Debug, Default, TryFromPrimitive)]
#[repr(u8)]
pub enum AbilityCard {
    #[default]
    NoUse = 0,
    Random = 1,
    SelfCard = 2,
    AllCard = 3,
}

#[derive(Clone, Debug, Default, TryFromPrimitive)]
#[repr(u8)]
pub enum Barrier {
    #[default]
    NoBarrier = 0,
    ManualOnly = 1,
    LongTime = 2,
    ShortTime = 3,
}

#[derive(Clone, Debug, Default, Deserialize, CopyGetters, Setters, Serialize, new)]
#[repr(C)]
pub struct GameSettings {
    #[getset(get_copy = "pub", set = "pub")]
    common: u32,
    #[getset(get_copy = "pub", set = "pub")]
    p1: u32,
    #[getset(get_copy = "pub", set = "pub")]
    p2: u32,
}

impl GameSettings {
    pub fn time_limit(&self) -> TimeLimit {
        let value = (self.common & 0b0000_0111) as u8;
        value.try_into().unwrap_or_default()
    }
    pub fn set_time_limit(&mut self, time_limit: TimeLimit) {
        self.common = self.common & !0b0000_0111 | time_limit as u32;
    }

    pub fn round(&self) -> Round {
        let value = ((self.common & 0b0011_0000) >> 4) as u8;
        value.try_into().unwrap_or_default()
    }
    pub fn set_round(&mut self, round: Round) {
        self.common = self.common & !0b0011_0000 | (round as u32) << 4;
    }

    pub fn ability_card(&self) -> AbilityCard {
        let value = ((self.common & 0b1100_0000) >> 6) as u8;
        value.try_into().unwrap_or_default()
    }
    pub fn set_ability_card(&mut self, ability_card: AbilityCard) {
        self.common = self.common & !0b1100_0000 | (ability_card as u32) << 6;
    }

    pub fn p1_life(&self) -> u32 {
        self.p1 & 0b0000_0111
    }
    pub fn set_p1_life(&mut self, life: u32) {
        self.p1 = self.p1 & !0b0000_0111 | life;
    }

    pub fn p1_barrier(&self) -> Barrier {
        let value = ((self.p1 & 0b0001_1000) >> 3) as u8;
        value.try_into().unwrap_or_default()
    }
    pub fn set_p1_barrier(&mut self, barrier: Barrier) {
        self.p1 = self.p1 & !0b0001_1000 | (barrier as u32) << 3;
    }

    pub fn p2_life(&self) -> u32 {
        self.p2 & 0b0000_0111
    }
    pub fn set_p2_life(&mut self, life: u32) {
        self.p2 = self.p2 & !0b0000_0111 | life;
    }

    pub fn p2_barrier(&self) -> Barrier {
        let value = ((self.p2 & 0b0001_1000) >> 3) as u8;
        value.try_into().unwrap_or_default()
    }
    pub fn set_p2_barrier(&mut self, barrier: Barrier) {
        self.p2 = self.p2 & !0b0001_1000 | (barrier as u32) << 3;
    }
}

#[repr(C)]
pub struct Settings {
    _unknown1: [u8; 0xf0],
    game_settings: GameSettings,
}
