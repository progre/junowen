use std::mem::transmute;

use anyhow::{bail, Result};
use getset::{CopyGetters, Setters};

#[derive(Clone, Copy, PartialEq)]
pub struct Input(pub u32);

impl Input {
    pub const NULL:  /*-*/u16 = 0b00000000_00000000;
    pub const SHOT:  /*-*/u16 = 0b00000000_00000001;
    pub const CHARGE:/*-*/u16 = 0b00000000_00000010;
    pub const BOMB:  /*-*/u16 = 0b00000000_00000100;
    pub const SLOW:  /*-*/u16 = 0b00000000_00001000;
    pub const UP:    /*-*/u16 = 0b00000000_00010000;
    pub const DOWN:  /*-*/u16 = 0b00000000_00100000;
    pub const LEFT:  /*-*/u16 = 0b00000000_01000000;
    pub const RIGHT: /*-*/u16 = 0b00000000_10000000;
    pub const START: /*-*/u16 = 0b00000001_00000000;
}

impl From<u16> for Input {
    fn from(value: u16) -> Self {
        Self(value as u32)
    }
}

#[repr(C)]
pub struct InputDevice {
    _unknown1: [u8; 0x010],
    pub input: Input,
    pub prev_input: Input,
    _unknown2: [u8; 0x3bc],
}

#[repr(C)]
pub struct DevicesInput {
    _unknown1: [u8; 0x20],
    pub input_device_array: [InputDevice; 3 + 9],
    _unknown2: [u8; 0x14],
    pub p1_input_idx: u32,
    pub p2_input_idx: u32,
    // unknown continues...
}

impl DevicesInput {
    pub fn p1_input(&self) -> Input {
        self.input_device_array[self.p1_input_idx as usize].input
    }
    pub fn set_p1_input(&mut self, value: Input) {
        self.input_device_array[self.p1_input_idx as usize].input = value;
    }
    pub fn p1_prev_input(&self) -> Input {
        self.input_device_array[self.p1_input_idx as usize].prev_input
    }

    pub fn p2_input(&self) -> Input {
        self.input_device_array[self.p2_input_idx as usize].input
    }
    pub fn set_p2_input(&mut self, value: Input) {
        self.input_device_array[self.p2_input_idx as usize].input = value;
    }
    pub fn p2_prev_input(&self) -> Input {
        self.input_device_array[self.p2_input_idx as usize].prev_input
    }
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
    _unknown1: usize,
    func: usize,
    _unknown2: [u8; 0x18],
    arg: usize,
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

    pub fn find_menu(&self) -> Option<&'static Menu> {
        let arg = self.to_vec().iter().find(|item| item.id == 0x0a)?.arg as *const Menu;
        unsafe { arg.as_ref() }
    }
    pub fn find_menu_mut(&self) -> Option<&'static mut Menu> {
        let arg = self.to_vec().iter().find(|item| item.id == 0x0a)?.arg as *mut Menu;
        unsafe { arg.as_mut() }
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

#[repr(C)]
pub struct Battle {
    _unknown: [u8; 0x10],
    pub pre_frame: u32,
    pub frame: u32,
}

#[derive(CopyGetters, Setters)]
#[repr(C)]
pub struct BattlePlayer {
    _unknown1: [u8; 0x0c],
    #[get_copy = "pub"]
    character: u32,
    _unknown2: [u8; 0x80],
    #[getset(get_copy = "pub", set = "pub")]
    card: u32,
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

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum ScreenId {
    Loading,
    Title,
    BattleLoading,
    Option,
    ControllerSettings,
    BattleSettings,
    Unknown2,
    DifficultySelect,
    PlayerMatchupSelect,
    OnlineMenu,
    CharacterSelect,
    Unknown3,
    Unknown4,
    Unknown5,
    Unknown6,
    MusicRoom,
    Unknown7,
    Unknown8,
    Manual,
    Unknown9,
    Archievements,
}

#[repr(C)]
pub struct CharacterCursor {
    pub cursor: u32,
    pub prev_cursor: u32,
    _unknown1: [u8; 0xd0],
}

#[repr(C)]
pub struct Menu {
    _unknown1: [u8; 0x18],
    pub screen_id: ScreenId,
    _prev_screen_id: u32,
    _unknown2: u32,
    _unknown3: u32,
    _unknown4: u32,
    pub cursor: u32,
    _prev_cursor: u32,
    pub max_cursor: u32,
    _unknown5: [u8; 0xcc],
    pub p1_cursor: CharacterCursor,
    pub p2_cursor: CharacterCursor,
}
