mod app;
mod inputdevices;
pub mod th19_helpers;
mod th19_structs;

use std::{arch::asm, ffi::c_void, mem::transmute};

use anyhow::{anyhow, Result};
use windows::{
    core::Interface,
    Win32::{Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY},
};

pub use crate::memory_accessors::FnOfHookAssembly;
use crate::{
    hook,
    memory_accessors::{ExternalProcess, HookedProcess, MemoryAccessor},
    pointer, ptr_opt, u16_prop, u32_prop, value, value_ref,
};
pub use app::*;
pub use inputdevices::*;
pub use th19_structs::*;

pub type Fn002530 = extern "thiscall" fn(this: *const c_void);
pub type Fn009fa0 = extern "thiscall" fn(this: *const c_void, arg1: u32) -> u32;
pub type Fn012480 = extern "thiscall" fn(this: *const c_void, arg1: u32) -> u32;
pub type Fn0a9000 = extern "fastcall" fn(arg1: i32);
pub type Fn102ff0 = extern "fastcall" fn(arg1: *const c_void);
pub type Fn1049e0 = extern "fastcall" fn();
pub type Fn10f720 = extern "fastcall" fn();

extern "fastcall" fn dummy_from_02d1f0_007c() {
    unsafe {
        asm! {
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            //
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
        }
    }
}

extern "fastcall" fn dummy_from_0aba30_00fb() {
    unsafe {
        asm! {
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            //
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
        }
    }
}

extern "fastcall" fn dummy_from_0aba30_018e() {
    unsafe {
        asm! {
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            //
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
            "NOP",
        }
    }
}

pub type ApplyFn = Box<dyn FnOnce(&mut Th19)>;

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

    pub fn hook_on_waiting_online_vs_connection(
        &mut self,
        target: FnOfHookAssembly,
    ) -> (Option<FnOfHookAssembly>, ApplyFn) {
        const ADDR: usize = 0x02d1f0 + 0x007c;
        const SIZE: usize = 7;
        self.hook_assembly(ADDR, SIZE, dummy_from_02d1f0_007c, target)
    }

    hook!(0x0a9540 + 0x0175, hook_0a9540_0175, Fn0a9000);

    pub fn hook_on_input_players(
        &mut self,
        target: FnOfHookAssembly,
    ) -> (Option<FnOfHookAssembly>, ApplyFn) {
        const ADDR: usize = 0x0aba30 + 0x00fb;
        const SIZE: usize = 10;
        self.hook_assembly(ADDR, SIZE, dummy_from_0aba30_00fb, target)
    }

    pub fn hook_on_input_menu(
        &mut self,
        target: FnOfHookAssembly,
    ) -> (Option<FnOfHookAssembly>, ApplyFn) {
        const ADDR: usize = 0x0aba30 + 0x018e;
        const SIZE: usize = 5;
        self.hook_assembly(ADDR, SIZE, dummy_from_0aba30_018e, target)
    }

    hook!(0x107540 + 0x0046, hook_107540_0046, Fn012480);
    hook!(0x107540 + 0x0937, hook_107540_0937, Fn002530);

    hook!(0x11f870 + 0x034c, hook_11f870_034c, Fn1049e0);

    hook!(0x130ed0 + 0x03ec, hook_130ed0_03ec, Fn102ff0);

    hook!(0x13f9d0 + 0x0345, hook_13f9d0_0345, Fn10f720);
    hook!(0x13f9d0 + 0x0446, hook_13f9d0_0446, Fn009fa0);

    // -------------------------------------------------------------------------

    u32_prop!(0x1a2478, difficulty_cursor, set_difficulty_cursor);

    pointer!(0x_1ae3a0, input_devices, input_devices_mut, InputDevices);
    u16_prop!(0x1ae410, rand_seed1, set_rand_seed1); // 同一フレームでも変わる可能性あり ここを起点にdesyncする？
    u32_prop!(0x1ae414, rand_seed5, set_rand_seed5); // 公式にsyncしてない 遅いほうだけ書き換わる？0,0 時に複数回書き換わるが、その後は変わらない?
                                                     // 0x1ae418: unknown
    pointer!(0x_1ae41c, app, App);
    u16_prop!(0x1ae420, rand_seed4, set_rand_seed4); // frame 依存?
    u32_prop!(0x1ae424, rand_seed6, set_rand_seed6); // 公式にsyncしてない 書き換えが稀
    u16_prop!(0x1ae428, rand_seed3, set_rand_seed3); // frame 依存 0x1ae410 のコピー？
    u32_prop!(0x1ae42c, rand_seed7, set_rand_seed7); // frame 依存 インクリメント
    u16_prop!(0x1ae430, rand_seed2, set_rand_seed2); // frame 依存 0x1ae420 のコピー？
    u32_prop!(0x1ae434, rand_seed8, set_rand_seed8); // frame 依存 インクリメント
    ptr_opt!(0x_1ae464, game, Game);
    value!(0x200850, p1_input, Input);
    value!(0x200b10, p2_input, Input);
    value!(0x200dd0, menu_input, set_menu_input, Input);
    value!(0x200dd4, prev_menu_input, Input);
    value_ref!(0x207910, p1, p1_mut, Player);
    value_ref!(0x2079d0, p2, p2_mut, Player);

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
    pub fn game_settings_in_game(&self) -> Result<GameSettings> {
        self.game_settings_from(0x208350)
    }
    pub fn put_game_settings_in_game(&mut self, game_settings: &GameSettings) -> Result<()> {
        self.put_game_settings_to(0x208350, game_settings)
    }

    pub fn direct_3d_device(&self) -> Result<&'static IDirect3DDevice9> {
        let memory_accessor = self.hooked_process_memory_accessor();
        let p_p_direct_3d_device = memory_accessor.raw_ptr(0x208388) as *const *mut c_void;
        unsafe { IDirect3DDevice9::from_raw_borrowed(&*p_p_direct_3d_device) }
            .ok_or_else(|| anyhow!("IDirect3DDevice9::from_raw_borrowed failed"))
    }

    pub fn set_no_wait(&mut self, value: bool) {
        self.memory_accessor
            .write_u32(0x208498, if value { 0x00000001 } else { 0x80000000 })
            .unwrap();
    }

    pub fn game_settings_in_menu(&self) -> Result<GameSettings> {
        self.game_settings_from(0x208644)
    }
    pub fn put_game_settings_in_menu(&mut self, game_settings: &GameSettings) -> Result<()> {
        self.put_game_settings_to(0x208644, game_settings)
    }

    // -------------------------------------------------------------------------

    fn value<T>(&self, addr: usize) -> T
    where
        T: Copy,
    {
        let p_obj = self.hooked_process_memory_accessor().raw_ptr(addr) as *const T;
        unsafe { *p_obj }
    }
    fn set_value<T>(&mut self, addr: usize, value: T)
    where
        T: Copy,
    {
        let p_obj = self.hooked_process_memory_accessor_mut().raw_ptr(addr) as *mut T;
        unsafe { *p_obj = value };
    }

    fn value_ref<T>(&self, addr: usize) -> &'static T {
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

    fn hook_call(&mut self, addr: usize, target: usize) -> (usize, ApplyFn) {
        let memory_accessor = self.hooked_process_memory_accessor_mut();
        let old_target = memory_accessor.current_callback_of_hook_call(addr);
        (
            old_target,
            Box::new(move |zelf: &mut Th19| {
                let memory_accessor = zelf.hooked_process_memory_accessor_mut();
                let old_flag = memory_accessor
                    .virtual_protect(addr, 5, PAGE_EXECUTE_WRITECOPY)
                    .unwrap();
                let old = memory_accessor.hook_call(addr, target);
                assert!(old == old_target);
                memory_accessor.virtual_protect(addr, 5, old_flag).unwrap();
            }),
        )
    }

    fn hook_assembly(
        &mut self,
        addr: usize,
        size: usize,
        dummy_func: extern "fastcall" fn(),
        target: FnOfHookAssembly,
    ) -> (Option<FnOfHookAssembly>, ApplyFn) {
        let memory_accessor = self.hooked_process_memory_accessor();
        let old_target = memory_accessor.current_callback_of_hook_assembly(addr);
        (
            old_target,
            Box::new(move |zelf: &mut Th19| {
                let memory_accessor = zelf.hooked_process_memory_accessor_mut();

                let parent_old = memory_accessor
                    .virtual_protect(addr, size, PAGE_EXECUTE_WRITECOPY)
                    .unwrap();
                let my_old = memory_accessor
                    .virtual_protect_global(dummy_func as _, size + 5 + 6, PAGE_EXECUTE_WRITECOPY)
                    .unwrap();

                let old = memory_accessor.hook_assembly(addr, size, dummy_func, target as _);
                assert!(old == old_target);

                memory_accessor
                    .virtual_protect_global(dummy_func as _, size + 5 + 6, my_old)
                    .unwrap();
                memory_accessor
                    .virtual_protect(addr, size, parent_old)
                    .unwrap();
            }),
        )
    }

    fn game_settings_from(&self, addr: usize) -> Result<GameSettings> {
        let mut buffer = [0u8; 12];
        self.memory_accessor.read(addr, &mut buffer)?;
        Ok(unsafe { transmute(buffer) })
    }
    fn put_game_settings_to(&mut self, addr: usize, game_settings: &GameSettings) -> Result<()> {
        let buffer: &[u8; 12] = unsafe { transmute(game_settings) };
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
