mod find_process_id;
pub mod inject_dll;
pub mod memory_accessors;
pub mod win_api_wrappers;

use std::{
    ffi::c_void,
    mem::{size_of, transmute},
};

use anyhow::{anyhow, bail, Result};
use memory_accessors::{ExternalProcess, HookedProcess, MemoryAccessor};
use windows::{
    core::Interface,
    Win32::{Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY},
};

pub struct Th19 {
    memory_accessor: MemoryAccessor,
}

macro_rules! u16_prop {
    ($addr:expr, $getter:ident) => {
        pub fn $getter(&self) -> Result<u16> {
            self.memory_accessor.read_u16($addr)
        }
    };

    ($addr:expr, $getter:ident, $setter:ident) => {
        u16_prop!($addr, $getter);
        pub fn $setter(&self, value: u16) -> Result<()> {
            self.memory_accessor.write_u16($addr, value)
        }
    };
}

#[repr(C)] // 0x3d4
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
    id: usize,
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
    game_mains: &'static GameMainsLinkedList,
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

impl Th19 {
    pub fn new_external_process(exe_file: &str) -> Result<Self> {
        Ok(Self {
            memory_accessor: MemoryAccessor::ExternalProcess(ExternalProcess::new(exe_file)?),
        })
    }

    pub fn new_hooked_process(exe_file: &str) -> Result<Self> {
        Ok(Self {
            memory_accessor: MemoryAccessor::HookedProcess(HookedProcess::new(exe_file)?),
        })
    }

    pub fn hook_0a96b5(&self, target: usize) -> Result<usize> {
        self.hook_call(0x0a96b5, target)
    }

    pub fn hook_0abb2b(&self, target: usize) -> Result<usize> {
        self.hook_call(0x0abb2b, target)
    }

    pub fn hook_120db5(&self, target: usize) -> Result<usize> {
        self.hook_call(0x120db5, target)
    }

    pub fn hook_13fe16(&self, target: usize) -> Result<usize> {
        self.hook_call(0x13fe16, target)
    }

    pub fn hook_107260_0067(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107260 + 0x0067, target)
    }
    pub fn hook_107260_01ba(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107260 + 0x01ba, target)
    }

    pub fn hook_107540_0046(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107540 + 0x0046, target)
    }
    pub fn hook_107540_045c(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107540 + 0x045c, target)
    }
    pub fn hook_107540_07bf(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107540 + 0x07bf, target)
    }
    pub fn hook_107540_08a1(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107540 + 0x08a1, target)
    }
    pub fn hook_107540_0937(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107540 + 0x0937, target)
    }

    pub fn input_mut(&self) -> &'static mut Input {
        debug_assert_eq!(0x03d4, size_of::<InputDevice>());
        debug_assert_eq!(0x2e2c, size_of::<Input>());

        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hook_120db5 is only available for HookedProcess");
        };
        let p_p_input = memory_accessor.raw_ptr(0x1ae3a0) as *const *mut Input;
        unsafe { (*p_p_input).as_mut().unwrap() }
    }

    u16_prop!(0x1ae410, rand_seed1, set_rand_seed1);

    pub fn game(&self) -> &'static Game {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::game is only available for HookedProcess");
        };
        unsafe {
            (*(memory_accessor.raw_ptr(0x1ae41c) as *const *const Game))
                .as_ref()
                .unwrap()
        }
    }

    u16_prop!(0x1ae430, rand_seed2, set_rand_seed2);
    u16_prop!(0x200850, p1_input);
    u16_prop!(0x200b10, p2_input);

    pub fn game_mode(&self) -> Result<GameMode> {
        self.memory_accessor.read_u32(0x207a94)?.try_into()
    }

    pub fn player_matchup(&self) -> Result<PlayerMatchup> {
        self.memory_accessor.read_u32(0x207a98)?.try_into()
    }

    // 0x208260 Game
    pub fn battle_settings_in_game(&self) -> Result<BattleSettings> {
        self.battle_settings_from(0x208350)
    }
    pub fn set_battle_settings_in_game(&self, battle_settings: &BattleSettings) -> Result<()> {
        self.set_battle_settings_to(0x208350, battle_settings)
    }

    pub fn direct_3d_device(&self) -> Result<&'static IDirect3DDevice9> {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::direct_3d_device is only available for HookedProcess");
        };
        unsafe {
            IDirect3DDevice9::from_raw_borrowed(
                &*(memory_accessor.raw_ptr(0x208388) as *const *mut c_void),
            )
        }
        .ok_or_else(|| anyhow!("IDirect3DDevice9::from_raw_borrowed failed"))
    }

    pub fn battle_settings_in_menu(&self) -> Result<BattleSettings> {
        self.battle_settings_from(0x208644)
    }
    pub fn set_battle_settings_in_menu(&self, battle_settings: &BattleSettings) -> Result<()> {
        self.set_battle_settings_to(0x208644, battle_settings)
    }

    // -------------------------------------------------------------------------

    pub fn is_network_mode(&self) -> bool {
        if self.game_mode().unwrap() == GameMode::Story {
            return false;
        }
        // VS Mode 最初の階層では player_matchup がまだセットされないので、オンライン用メイン関数がセットされているかどうかで判断する
        self.game()
            .game_mains
            .to_vec()
            .iter()
            .any(|item| item.id == 3 || item.id == 4)
    }

    fn hook_call(&self, addr: usize, target: usize) -> Result<usize> {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hook_addr is only available for HookedProcess");
        };
        let old = memory_accessor.virtual_protect(addr, 5, PAGE_EXECUTE_WRITECOPY)?;
        let original = memory_accessor.hook_call(addr, target);
        memory_accessor.virtual_protect(addr, 5, old)?;
        Ok(original)
    }

    fn battle_settings_from(&self, addr: usize) -> Result<BattleSettings> {
        let mut buffer = [0u8; 12];
        self.memory_accessor.read(addr, &mut buffer)?;
        Ok(unsafe { transmute(buffer) })
    }
    fn set_battle_settings_to(&self, addr: usize, battle_settings: &BattleSettings) -> Result<()> {
        let buffer: &[u8; 12] = unsafe { transmute(battle_settings) };
        self.memory_accessor.write(addr, buffer)
    }
}
