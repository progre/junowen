use std::mem::transmute;

use anyhow::{bail, Result};
use getset::{Getters, MutGetters};

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
