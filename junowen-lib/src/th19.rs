pub mod structs;
pub mod th19_helpers;

use std::{arch::asm, ffi::c_void, mem::transmute};

use anyhow::Result;
use tracing::debug;
use windows::Win32::{
    Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY,
};

pub use crate::memory_accessors::FnOfHookAssembly;
use crate::{
    hook, hook_todo,
    memory_accessors::{ExternalProcess, HookedProcess, MemoryAccessor},
    pointer, ptr_opt, u32_prop, u32_prop_todo, value_ref,
};

use self::structs::{
    app::App,
    input_devices::{Input, InputDevices},
    others::{RenderingText, RoundFrame, VSMode, WindowInner},
    selection::Selection,
    settings::GameSettings,
};

pub type Fn002530 = extern "thiscall" fn(*const c_void);
pub type Fn009fa0 = extern "thiscall" fn(*const c_void, u32) -> u32;
pub type Fn011560 = extern "thiscall" fn(*const Selection) -> u8;
pub type Fn012480 = extern "thiscall" fn(*const c_void, u32) -> u32;
pub type Fn0a9000 = extern "thiscall" fn(*const c_void);
pub type Fn0b7d40 = extern "thiscall" fn(*const c_void, *const c_void);
pub type Fn0d5ae0 = extern "thiscall" fn(*const c_void, *mut RenderingText) -> u32;
pub type Fn0d6e10 = extern "thiscall" fn(*const c_void, *const c_void) -> u32;
pub type Fn102ff0 = extern "fastcall" fn(*const c_void);
pub type Fn1049e0 = extern "fastcall" fn();
pub type Fn10f720 = extern "fastcall" fn();

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
        &self,
        _target: FnOfHookAssembly,
    ) -> (Option<FnOfHookAssembly>, ApplyFn) {
        todo!();
    }

    hook!(0x0a9540 + 0x0175, hook_0a9540_0175, Fn0a9000);

    pub fn hook_on_input_players(
        &self,
        target: FnOfHookAssembly,
    ) -> (Option<FnOfHookAssembly>, ApplyFn) {
        const ADDR: usize = 0x0b8760 + 0x012a;
        const SIZE: usize = 10;
        self.hook_assembly(ADDR, SIZE, dummy_from_0aba30_00fb, target)
    }
    pub fn hook_on_input_menu(
        &self,
        target: FnOfHookAssembly,
    ) -> (Option<FnOfHookAssembly>, ApplyFn) {
        const ADDR: usize = 0x0b8760 + 0x01bc;
        const SIZE: usize = 5;
        self.hook_assembly(ADDR, SIZE, dummy_from_0aba30_018e, target)
    }

    /// 01: カード送り
    /// 02: ピチューン
    /// 07: 決定
    /// 08: 決定(重)
    /// 09: キャンセル
    /// 0a: 選択
    /// 10: ブブー
    /// 11: エクステンド
    /// 1f: ガシャコン
    /// 2e: ボム回収効果音
    /// 57: ガシャコン(重)
    pub fn play_sound(&self, this: *const c_void, id: u32, arg2: u32) {
        type Fn = extern "thiscall" fn(*const c_void, u32, u32);
        const ADDR: usize = 0x0bb8d0;
        let ptr = self.hooked_process_memory_accessor().raw_ptr(ADDR);
        (unsafe { transmute::<*const c_void, Fn>(ptr) })(this, id, arg2)
    }

    hook!(0x0cc5a0 + 0x012c, hook_0bed70_00fc, Fn0b7d40);

    pub fn render_text(&self, text_renderer: *const c_void, text: &RenderingText) -> u32 {
        const ADDR: usize = 0x0e5850;
        let ptr = self.hooked_process_memory_accessor().raw_ptr(ADDR);
        (unsafe { transmute::<*const c_void, Fn0d5ae0>(ptr) })(text_renderer, text as *const _ as _)
    }

    hook!(0x0e6b61 + 0x0038, hook_0d6e10_0039, Fn0d5ae0);

    hook!(0x0e6ef0 + 0x0008, hook_0d7180_0008, Fn0d6e10);

    hook_todo!(0x107540 + 0x0046, hook_107540_0046, Fn012480);
    hook_todo!(0x107540 + 0x0937, hook_107540_0937, Fn002530);

    hook!(0x132CF0 + 0x029f, hook_11f870_034c, Fn1049e0);

    hook!(0x137d40 + 0x0103, hook_1243f0_00f9, Fn011560);
    hook!(0x137d40 + 0x0338, hook_1243f0_0320, Fn011560);

    hook_todo!(0x130ed0 + 0x03ec, hook_130ed0_03ec, Fn102ff0);

    hook!(0x156340 + 0x0475, hook_13f9d0_0345, Fn10f720);
    hook!(0x156340 + 0x056e, hook_13f9d0_0446, Fn009fa0);

    // -------------------------------------------------------------------------

    u32_prop!(0x1c5508, difficulty_cursor, set_difficulty_cursor);
    u32_prop!(0x1c6624, rand_seed1, set_rand_seed1);
    u32_prop!(0x1c6634, rand_seed2, set_rand_seed2);
    u32_prop!(0x1c663c, rand_seed3, set_rand_seed3);
    // 隊列
    u32_prop!(0x1c664c, rand_seed4, set_rand_seed4);

    pointer!(0x_1d19b0, input_devices, input_devices_mut, InputDevices);
    pointer!(0x_1d1a24, app, app_mut, App);
    ptr_opt!(0x_1d1a54, round_frame, RoundFrame);
    pointer!(0x_1d1a60 + 0x2c, selection, selection_mut, Selection);
    pointer!(0x_1d1c00, vs_mode, VSMode);

    value_ref!(0x223e40, p1_input, Input);
    value_ref!(0x224100, p2_input, Input);
    value_ref!(0x2243c0, menu_input, menu_input_mut, Input);
    value_ref!(0x225440, sound_manager, c_void);

    pub fn game_settings_in_game(&self) -> Result<GameSettings> {
        self.game_settings_from(0x22bb10)
    }
    pub fn put_game_settings_in_game(&mut self, game_settings: &GameSettings) -> Result<()> {
        self.put_game_settings_to(0x22bb10, game_settings)
    }

    pub fn direct_3d_device(&self) -> Result<&'static IDirect3DDevice9> {
        todo!();
    }

    pub fn no_wait(&mut self) -> bool {
        self.memory_accessor.read_u32(0x22bc58).unwrap() == 0x00000001
    }
    pub fn set_no_wait(&mut self, value: bool) {
        debug!("set_no_wait: {}", value);
        self.memory_accessor
            .write_u32(0x22bc58, if value { 0x00000001 } else { 0x80000000 })
            .unwrap();
    }

    pub fn game_settings_in_menu(&self) -> Result<GameSettings> {
        self.game_settings_from(0x22be04)
    }
    pub fn put_game_settings_in_menu(&mut self, game_settings: &GameSettings) -> Result<()> {
        self.put_game_settings_to(0x22be04, game_settings)
    }

    // 0x208380+0x0910
    // 04: menu, 07: game
    u32_prop_todo!(0x208c90, scene);

    value_ref!(0x22ee90, window_inner, WindowInner);

    // -------------------------------------------------------------------------

    fn _value<T>(&self, addr: usize) -> T
    where
        T: Copy,
    {
        let p_obj = self.hooked_process_memory_accessor().raw_ptr(addr) as *const T;
        unsafe { *p_obj }
    }
    fn _set_value<T>(&mut self, addr: usize, value: T)
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

    fn hook_call(&self, addr: usize, target: usize) -> (usize, ApplyFn) {
        let memory_accessor = self.hooked_process_memory_accessor();
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
        &self,
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
        Ok(unsafe { transmute::<[u8; 12], GameSettings>(buffer) })
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
