mod helper;
mod in_game_lobby;
mod session;
mod state;
mod tracing_helper;

use std::{ffi::c_void, path::PathBuf, slice};

use junowen_lib::{
    hook_utils::WELL_KNOWN_VERSION_HASHES, Fn009fa0, Fn011560, Fn0b7d40, Fn0d5ae0, Fn0d6e10,
    Fn1049e0, Fn10f720, FnOfHookAssembly, RenderingText, Selection, Th19,
};
use state::State;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, HMODULE, MAX_PATH},
        Graphics::Direct3D9::IDirect3D9,
        System::{
            Console::AllocConsole, LibraryLoader::GetModuleFileNameW,
            SystemServices::DLL_PROCESS_ATTACH,
        },
    },
};

static mut MODULE: HMODULE = HMODULE(0);
static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    old_on_input_players: Option<FnOfHookAssembly>,
    old_on_input_menu: Option<FnOfHookAssembly>,
    old_fn_from_0bed70_00fc: Fn0b7d40,
    old_fn_from_0d6e10_0039: Fn0d5ae0,
    old_fn_from_0d7180_0008: Fn0d6e10,
    old_fn_from_11f870_034c: Fn1049e0,
    old_fn_from_1243f0_00f9: Fn011560,
    old_fn_from_1243f0_0320: Fn011560,
    old_fn_from_13f9d0_0345: Fn10f720,
    old_fn_from_13f9d0_0446: Fn009fa0,
}

fn props() -> &'static Props {
    unsafe { PROPS.as_ref().unwrap() }
}

fn state() -> &'static State {
    unsafe { STATE.as_ref().unwrap() }
}
fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

extern "fastcall" fn on_input_players() {
    state_mut().on_input_players();

    if let Some(func) = props().old_on_input_players {
        func()
    }
}

extern "fastcall" fn on_input_menu() {
    state_mut().on_input_menu();

    if let Some(func) = props().old_on_input_menu {
        func()
    }
}

extern "thiscall" fn render_object(this: *const c_void, obj: *const c_void) {
    state().render_object(props().old_fn_from_0bed70_00fc, this, obj);
}

extern "thiscall" fn render_text(text_renderer: *const c_void, text: *mut RenderingText) -> u32 {
    let text = unsafe { text.as_mut().unwrap() };
    state_mut().render_text(props().old_fn_from_0d6e10_0039, text_renderer, text)
}

extern "thiscall" fn on_render_texts(text_renderer: *const c_void, arg: *const c_void) -> u32 {
    let ret = (props().old_fn_from_0d7180_0008)(text_renderer, arg);
    state_mut().on_render_texts(text_renderer);
    ret
}

extern "fastcall" fn on_round_over() {
    (props().old_fn_from_11f870_034c)();

    state_mut().on_round_over();
}

/// for pause menu online vs view
extern "thiscall" fn fn_from_1243f0_00f9(this: *const Selection) -> u8 {
    state().is_online_vs(this, props().old_fn_from_1243f0_00f9)
}

/// for pause menu online vs view
extern "thiscall" fn fn_from_1243f0_0320(this: *const Selection) -> u8 {
    state().is_online_vs(this, props().old_fn_from_1243f0_0320)
}

extern "fastcall" fn on_rewrite_controller_assignments() {
    // NOTE: old_fn() modifies th19 outside of Rust.
    //       This reference makes Rust aware of the change.
    state_mut().on_rewrite_controller_assignments(|_: &mut Th19| props().old_fn_from_13f9d0_0345);
}

extern "thiscall" fn on_loaded_game_settings(this: *const c_void, arg1: u32) -> u32 {
    state_mut().on_loaded_game_settings();

    (props().old_fn_from_13f9d0_0446)(this, arg1)
}

fn check_version(hash: &[u8]) -> bool {
    WELL_KNOWN_VERSION_HASHES
        .all_v100a()
        .iter()
        .any(|&valid_hash| valid_hash == hash)
}

fn init(module: HMODULE) {
    if cfg!(debug_assertions) {
        let _ = unsafe { AllocConsole() };
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let dll_path = {
        let mut buf = [0u16; MAX_PATH as usize];
        if unsafe { GetModuleFileNameW(module, &mut buf) } == 0 {
            panic!();
        }
        unsafe { PCWSTR::from_raw(buf.as_ptr()).to_string() }.unwrap()
    };
    let dll_path = PathBuf::from(dll_path);
    tracing_helper::init_tracing(
        dll_path.parent().unwrap().to_string_lossy().as_ref(),
        &format!("{}.log", dll_path.file_stem().unwrap().to_string_lossy()),
        false,
    );

    let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();

    let (old_on_input_players, apply_hook_on_input_players) =
        th19.hook_on_input_players(on_input_players);
    let (old_on_input_menu, apply_hook_on_input_menu) = th19.hook_on_input_menu(on_input_menu);
    let (old_fn_from_0bed70_00fc, apply_hook_0bed70_00fc) = th19.hook_0bed70_00fc(render_object);
    let (old_fn_from_0d6e10_0039, apply_hook_0d6e10_0039) = th19.hook_0d6e10_0039(render_text);
    let (old_fn_from_0d7180_0008, apply_hook_0d7180_0008) = th19.hook_0d7180_0008(on_render_texts);
    let (old_fn_from_11f870_034c, apply_hook_11f870_034c) = th19.hook_11f870_034c(on_round_over);
    let (old_fn_from_1243f0_00f9, apply_hook_1243f0_00f9) =
        th19.hook_1243f0_00f9(fn_from_1243f0_00f9);
    let (old_fn_from_1243f0_0320, apply_hook_1243f0_0320) =
        th19.hook_1243f0_0320(fn_from_1243f0_0320);
    let (old_fn_from_13f9d0_0345, apply_hook_13f9d0_0345) =
        th19.hook_13f9d0_0345(on_rewrite_controller_assignments);
    let (old_fn_from_13f9d0_0446, apply_hook_13f9d0_0446) =
        th19.hook_13f9d0_0446(on_loaded_game_settings);

    unsafe {
        PROPS = Some(Props {
            old_on_input_players,
            old_on_input_menu,
            old_fn_from_0bed70_00fc,
            old_fn_from_0d6e10_0039,
            old_fn_from_0d7180_0008,
            old_fn_from_11f870_034c,
            old_fn_from_1243f0_00f9,
            old_fn_from_1243f0_0320,
            old_fn_from_13f9d0_0345,
            old_fn_from_13f9d0_0446,
        });
        STATE = Some(State::new(th19));
    }
    let th19 = &mut state_mut().th19_mut();
    apply_hook_on_input_players(th19);
    apply_hook_on_input_menu(th19);
    apply_hook_0bed70_00fc(th19);
    apply_hook_0d6e10_0039(th19);
    apply_hook_0d7180_0008(th19);
    apply_hook_11f870_034c(th19);
    apply_hook_1243f0_00f9(th19);
    apply_hook_1243f0_0320(th19);
    apply_hook_13f9d0_0345(th19);
    apply_hook_13f9d0_0446(th19);
}

#[no_mangle]
pub extern "stdcall" fn DllMain(inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        unsafe { MODULE = inst_dll.into() };
    }
    true
}

/// # Safety
/// The size allocated by `hash` must be indicated by `length`.
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn CheckVersion(hash: *const u8, length: usize) -> bool {
    let hash = unsafe { slice::from_raw_parts(hash, length) };
    check_version(hash)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn Initialize(_direct_3d: *const IDirect3D9) -> bool {
    init(unsafe { MODULE });

    true
}
