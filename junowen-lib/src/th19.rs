mod th19_helpers;
mod th19_structs;

use std::{ffi::c_void, mem::transmute};

use anyhow::{anyhow, Result};
use windows::{
    core::Interface,
    Win32::{Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY},
};

use crate::{
    memory_accessors::{ExternalProcess, HookedProcess, MemoryAccessor},
    pointer, ptr_opt, u16_prop, value,
};
pub use th19_helpers::*;
pub use th19_structs::*;

pub type Fn002530 = extern "thiscall" fn(this: *const c_void);
pub type Fn009fa0 = extern "thiscall" fn(this: *const c_void, arg1: u32) -> u32;
pub type Fn012480 = extern "thiscall" fn(this: *const c_void, arg1: u32) -> u32;
pub type Fn0a9000 = extern "fastcall" fn(arg1: i32);
pub type FnFrom0aba30_00fb = extern "fastcall" fn() -> u32;

pub struct Th19 {
    memory_accessor: MemoryAccessor,
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

    pub fn hook_0a9540_0175(&mut self, target: Fn0a9000) -> Result<Fn0a9000> {
        Ok(unsafe { transmute(self.hook_call(0x0a9540 + 0x0175, target as _)?) })
    }

    pub fn hook_0aba30_00fb(
        &mut self,
        target: FnFrom0aba30_00fb,
    ) -> Result<Option<FnFrom0aba30_00fb>> {
        let addr = 0x0aba30 + 0x00fb;
        let MemoryAccessor::HookedProcess(memory_accessor) = &mut self.memory_accessor else {
            panic!("Th19::hook_0abb2b is only available for HookedProcess");
        };
        let old = memory_accessor.virtual_protect(addr, 14, PAGE_EXECUTE_WRITECOPY)?;
        let old_addr = memory_accessor.hook_assembly(addr, 14, target as _);
        memory_accessor.virtual_protect(addr, 14, old)?;
        Ok(old_addr.map(|old_addr| unsafe { transmute(old_addr) }))
    }

    pub fn hook_107540_0046(&mut self, target: Fn012480) -> Result<Fn012480> {
        Ok(unsafe { transmute(self.hook_call(0x107540 + 0x0046, target as _)?) })
    }
    pub fn hook_107540_0937(&mut self, target: Fn002530) -> Result<Fn002530> {
        Ok(unsafe { transmute(self.hook_call(0x107540 + 0x0937, target as _)?) })
    }

    pub fn hook_13f9d0_0446(&mut self, target: Fn009fa0) -> Result<Fn009fa0> {
        Ok(unsafe { transmute(self.hook_call(0x13f9d0 + 0x0446, target as _)?) })
    }

    // -------------------------------------------------------------------------

    pointer!(0x_1ae3a0, input, input_mut, DevicesInput);
    u16_prop!(0x1ae410, rand_seed1, set_rand_seed1);
    pointer!(0x_1ae41c, game, Game);
    u16_prop!(0x1ae430, rand_seed2, set_rand_seed2);
    ptr_opt!(0x_1ae464, battle, Battle);
    u16_prop!(0x200850, p1_input);
    u16_prop!(0x200b10, p2_input);
    value!(0x207910, battle_p1, battle_p1_mut, BattlePlayer);
    value!(0x2079d0, battle_p2, battle_p2_mut, BattlePlayer);

    pub fn difficulty(&self) -> Result<Difficulty> {
        self.memory_accessor.read_u32(0x207a90)?.try_into()
    }
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
    pub fn put_battle_settings_in_game(&mut self, battle_settings: &BattleSettings) -> Result<()> {
        self.put_battle_settings_to(0x208350, battle_settings)
    }

    pub fn direct_3d_device(&self) -> Result<&'static IDirect3DDevice9> {
        let memory_accessor = self.hooked_process_memory_accessor();
        let p_p_direct_3d_device = memory_accessor.raw_ptr(0x208388) as *const *mut c_void;
        unsafe { IDirect3DDevice9::from_raw_borrowed(&*p_p_direct_3d_device) }
            .ok_or_else(|| anyhow!("IDirect3DDevice9::from_raw_borrowed failed"))
    }

    pub fn battle_settings_in_menu(&self) -> Result<BattleSettings> {
        self.battle_settings_from(0x208644)
    }
    pub fn put_battle_settings_in_menu(&mut self, battle_settings: &BattleSettings) -> Result<()> {
        self.put_battle_settings_to(0x208644, battle_settings)
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

    // -------------------------------------------------------------------------

    fn value<T>(&self, addr: usize) -> &'static T {
        let p_obj = self.hooked_process_memory_accessor().raw_ptr(addr) as *const T;
        unsafe { p_obj.as_ref().unwrap() }
    }
    fn value_mut<T>(&mut self, addr: usize) -> &'static mut T {
        let p_obj = self.hooked_process_memory_accessor_mut().raw_ptr(addr) as *mut T;
        unsafe { p_obj.as_mut().unwrap() }
    }

    fn pointer<T>(&self, addr: usize) -> Option<&'static T> {
        let p_p_obj = self.hooked_process_memory_accessor().raw_ptr(addr) as *const *const T;
        unsafe { (*p_p_obj).as_ref() }
    }
    fn pointer_mut<T>(&mut self, addr: usize) -> Option<&'static mut T> {
        let p_p_obj = self.hooked_process_memory_accessor_mut().raw_ptr(addr) as *const *mut T;
        unsafe { (*p_p_obj).as_mut() }
    }

    fn hook_call(&mut self, addr: usize, target: usize) -> Result<usize> {
        let memory_accessor = self.hooked_process_memory_accessor_mut();
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
    fn put_battle_settings_to(
        &mut self,
        addr: usize,
        battle_settings: &BattleSettings,
    ) -> Result<()> {
        let buffer: &[u8; 12] = unsafe { transmute(battle_settings) };
        self.memory_accessor.write(addr, buffer)
    }

    fn hooked_process_memory_accessor(&self) -> &HookedProcess {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hooked_process_memory_accessor is only available for HookedProcess");
        };
        memory_accessor
    }
    fn hooked_process_memory_accessor_mut(&mut self) -> &mut HookedProcess {
        let MemoryAccessor::HookedProcess(memory_accessor) = &mut self.memory_accessor else {
            panic!("Th19::hooked_process_memory_accessor_mut is only available for HookedProcess");
        };
        memory_accessor
    }
}
