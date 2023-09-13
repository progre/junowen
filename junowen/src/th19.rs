mod th19_structs;

use std::{
    ffi::c_void,
    mem::{size_of, transmute},
};

use anyhow::{anyhow, Result};
use windows::{
    core::Interface,
    Win32::{Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY},
};

use crate::memory_accessors::{ExternalProcess, HookedProcess, MemoryAccessor};
pub use th19_structs::*;

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

    pub fn hook_0a9540_0175(&self, target: extern "fastcall" fn(arg1: i32)) -> Result<usize> {
        self.hook_call(0x0a9540 + 0x0175, target as _)
    }

    pub fn hook_0aba30_00fb(&self, target: extern "fastcall" fn() -> u32) -> Result<()> {
        let addr = 0x0aba30 + 0x00fb;
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hook_0abb2b is only available for HookedProcess");
        };
        let old = memory_accessor.virtual_protect(addr, 14, PAGE_EXECUTE_WRITECOPY)?;
        memory_accessor.hook_assembly(addr, 14, target as _);
        memory_accessor.virtual_protect(addr, 14, old)?;
        Ok(())
    }

    pub fn hook_107260_0067(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107260 + 0x0067, target)
    }
    pub fn hook_107260_01ba(&self, target: usize) -> Result<usize> {
        self.hook_call(0x107260 + 0x01ba, target)
    }

    pub fn hook_107540_0046(
        &self,
        target: extern "thiscall" fn(this: *const c_void, arg1: u32) -> u32,
    ) -> Result<usize> {
        self.hook_call(0x107540 + 0x0046, target as _)
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
    pub fn hook_107540_0937(
        &self,
        target: extern "thiscall" fn(this: *const c_void),
    ) -> Result<usize> {
        self.hook_call(0x107540 + 0x0937, target as _)
    }

    pub fn hook_120ca0_0115(&self, target: usize) -> Result<usize> {
        self.hook_call(0x120ca0 + 0x0115, target)
    }

    pub fn hook_13f9d0_0446(
        &self,
        target: extern "thiscall" fn(this: *const c_void, arg1: u32) -> u32,
    ) -> Result<usize> {
        self.hook_call(0x13f9d0 + 0x0446, target as _)
    }

    // -------------------------------------------------------------------------

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
