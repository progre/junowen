use std::{
    ffi::{CStr, FromBytesUntilNulError},
    fmt,
    mem::transmute,
};

use anyhow::{bail, Result};
use derivative::Derivative;
use derive_new::new;
use getset::{CopyGetters, Getters, MutGetters, Setters};
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

#[derive(Debug)]
#[repr(C)]
pub struct RoundFrame {
    _unknown: [u8; 0x10],
    pub pre_frame: u32,
    pub frame: u32,
}

impl RoundFrame {
    pub fn is_first_frame(&self) -> bool {
        self.pre_frame == 0xffffffff && self.frame == 0
    }
}

#[derive(CopyGetters)]
#[repr(C)]
pub struct VSMode {
    _unknown1: [u8; 0x02E868],
    _unknown2: [u8; 0x08],
    _unknown3: [u8; 0x58],
    player_name: [u8; 0x22],
    room_name: [u8; 0x22],
    _unknown4: [u8; 0x0108],
    /// Readonly
    #[get_copy = "pub"]
    p1_card: u8, // +2ea14h
    /// Readonly
    #[get_copy = "pub"]
    p2_card: u8,
    // unknown remains...
}

impl VSMode {
    pub fn player_name(&self) -> &str {
        CStr::from_bytes_until_nul(&self.player_name)
            .unwrap_or_default()
            .to_str()
            .unwrap()
    }

    pub fn room_name(&self) -> &str {
        CStr::from_bytes_until_nul(&self.room_name)
            .unwrap_or_default()
            .to_str()
            .unwrap()
    }
}

/// length=c0
#[repr(C)]
pub struct Player {
    _unknown1: [u8; 0x0c],
    /// NOT available on player select screen
    pub character: u32,
    _unknown2: [u8; 0x80],
    /// Available on player select screen
    pub card: u32,
    _unknown3: [u8; 0x2c],
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
    Lunatic,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self::Normal
    }
}

impl TryFrom<u32> for Difficulty {
    type Error = anyhow::Error;
    fn try_from(value: u32) -> Result<Self> {
        if !(0..4).contains(&value) {
            bail!("Invalid Difficulty: {}", value);
        }
        Ok(unsafe { transmute(value) })
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum GameMode {
    Story,
    Unused,
    Versus,
}

impl TryFrom<u32> for GameMode {
    type Error = anyhow::Error;
    fn try_from(value: u32) -> Result<Self> {
        if !(0..3).contains(&value) {
            bail!("Invalid GameMode: {}", value);
        }
        Ok(unsafe { transmute(value) })
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum PlayerMatchup {
    HumanVsHuman,
    HumanVsCpu,
    CpuVsCpu,
    YoukaiVsYoukai,
}

impl Default for PlayerMatchup {
    fn default() -> Self {
        Self::HumanVsHuman
    }
}

impl TryFrom<u32> for PlayerMatchup {
    type Error = anyhow::Error;
    fn try_from(value: u32) -> Result<Self> {
        if !(0..4).contains(&value) {
            bail!("Invalid PlayerMatchup: {}", value);
        }
        Ok(unsafe { transmute(value) })
    }
}

#[derive(Getters, MutGetters)]
#[repr(C)]
pub struct Selection {
    #[getset(get = "pub", get_mut = "pub")]
    p1: Player,
    #[getset(get = "pub", get_mut = "pub")]
    p2: Player,
    pub difficulty: Difficulty,
    pub game_mode: GameMode,
    pub player_matchup: PlayerMatchup,
}

#[derive(Derivative)]
#[derivative(Default)]
#[repr(C)]
pub struct RenderingText {
    #[derivative(Default(value = "[0u8; 256]"))]
    raw_text: [u8; 256],
    pub x: f32,
    pub y: f32,
    pub _unknown1: u32,
    /// 0xaarrggbb
    #[derivative(Default(value = "0xffffffff"))]
    pub color: u32,
    #[derivative(Default(value = "1.0"))]
    pub scale_x: f32,
    #[derivative(Default(value = "1.0"))]
    pub scale_y: f32,
    /// radian
    pub rotate: f32,
    pub _unknown2: [u8; 0x08],
    pub font_type: u32,
    pub drop_shadow: bool,
    pub _padding_drop_shadow: [u8; 0x03],
    pub _unknown3: u32,
    pub hide: u32,
    /// 0: center, 1: left, 2: right
    #[derivative(Default(value = "1"))]
    pub horizontal_align: u32,
    /// 0: center, 1: top, 2: bottom
    #[derivative(Default(value = "1"))]
    pub vertical_align: u32,
}

impl fmt::Debug for RenderingText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderingText")
            .field("text", &CStr::from_bytes_until_nul(&self.raw_text))
            .field("x", &self.x)
            .field("y", &self.y)
            .field("_unknown1", &self._unknown1)
            .field("color", &format!("{:x}", self.color))
            .field("scale_x", &self.scale_x)
            .field("scale_y", &self.scale_y)
            .field("rotate", &self.rotate)
            .field("_unknown2", &self._unknown2)
            .field("font_type", &self.font_type)
            .field("drop_shadow", &self.drop_shadow)
            .field("_padding_drop_shadow", &self._padding_drop_shadow)
            .field("_unknown3", &self._unknown3)
            .field("hide", &self.hide)
            .field("horizontal_align", &self.horizontal_align)
            .field("vertical_align", &self.vertical_align)
            .finish()
    }
}

impl RenderingText {
    pub fn text(&self) -> Result<&CStr, FromBytesUntilNulError> {
        CStr::from_bytes_until_nul(&self.raw_text)
    }

    pub fn set_text(&mut self, text: &[u8]) {
        let mut raw_text = [0u8; 256];
        raw_text[0..(text.len())].copy_from_slice(text);
        self.raw_text = raw_text;
    }
}
