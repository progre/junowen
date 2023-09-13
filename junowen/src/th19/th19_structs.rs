use std::mem::transmute;

use anyhow::{bail, Result};

#[repr(C)]
pub struct InputDevice {
    pub input: u32,
    _unknown2: [u8; 0x3d0],
}

#[repr(C)]
pub struct Input {
    pub _unknown1: [u8; 0x30],
    pub input_device_array: [InputDevice; 3 + 9],
    _unknown2: u32,
    pub p1_input_idx: u32,
    pub p2_input_idx: u32,
    // unknown continues...
}

#[derive(Default)]
#[repr(C)]
pub struct BattleSettings {
    pub common: u32,
    pub p1: u32,
    pub p2: u32,
}

#[repr(C)]
pub struct Settings {
    _unknown1: [u8; 0xf0],
    battle_settings: BattleSettings,
}

#[derive(Debug)]
#[repr(C)]
pub struct GameMainsLinkedListItem {
    pub id: usize,
    _unknown2: usize,
    func: usize,
}

#[repr(C)]
pub struct GameMainsLinkedList {
    item: *const GameMainsLinkedListItem,
    next: *const GameMainsLinkedList,
}

impl GameMainsLinkedList {
    #[must_use]
    pub fn len(&self) -> usize {
        let mut len = 0;
        let mut p = self as *const GameMainsLinkedList;
        loop {
            len += 1;
            p = unsafe { (*p).next };
            if p.is_null() {
                return len;
            }
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_vec(&self) -> Vec<&GameMainsLinkedListItem> {
        let mut vec = Vec::new();
        let mut list = self as *const Self;
        loop {
            if list.is_null() {
                return vec;
            }
            vec.push(unsafe { (*list).item.as_ref().unwrap() });
            list = unsafe { (*list).next };
        }
    }
}

#[repr(C)]
pub struct Game {
    _unknown1: [u8; 0x18],
    pub game_mains: &'static GameMainsLinkedList,
}

#[derive(PartialEq)]
#[repr(u32)]
pub enum GameMode {
    Story,
    Unused,
    Versus,
}

impl TryFrom<u32> for GameMode {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        if !(0..=2).contains(&value) {
            bail!("Invalid GameMode: {}", value);
        }
        Ok(unsafe { transmute(value) })
    }
}

#[derive(PartialEq)]
#[repr(u32)]
pub enum PlayerMatchup {
    HumanVsHuman,
    HumanVsCpu,
    CpuVsCpu,
    YoukaiVsYoukai,
}

impl TryFrom<u32> for PlayerMatchup {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        if !(0..=3).contains(&value) {
            bail!("Invalid PlayerMatchup: {}", value);
        }
        Ok(unsafe { transmute(value) })
    }
}
