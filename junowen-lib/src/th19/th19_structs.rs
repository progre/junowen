use std::{
    ffi::{CStr, FromBytesUntilNulError},
    fmt,
    mem::transmute,
};

use anyhow::{bail, Result};
use derivative::Derivative;
use getset::{Getters, MutGetters};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[repr(C)]
pub struct GameSettings {
    pub common: u32,
    pub p1: u32,
    pub p2: u32,
}

#[repr(C)]
pub struct Settings {
    _unknown1: [u8; 0xf0],
    game_settings: GameSettings,
}

#[derive(Debug)]
#[repr(C)]
pub struct Game {
    _unknown: [u8; 0x10],
    pub pre_frame: u32,
    pub frame: u32,
}

impl Game {
    pub fn is_first_frame(&self) -> bool {
        self.pre_frame == 0xffffffff && self.frame == 0
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
    pub _unknown3: [u8; 8],
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
            .field("_unknown4", &self._unknown3)
            .field("horizontal_align", &self.horizontal_align)
            .field("vertical_align", &self.vertical_align)
            .finish()
    }
}

impl RenderingText {
    pub fn text(&mut self) -> Result<&CStr, FromBytesUntilNulError> {
        CStr::from_bytes_until_nul(&self.raw_text)
    }

    pub fn set_text(&mut self, text: &[u8]) {
        let mut raw_text = [0u8; 256];
        raw_text[0..(text.len())].copy_from_slice(text);
        self.raw_text = raw_text;
    }
}
