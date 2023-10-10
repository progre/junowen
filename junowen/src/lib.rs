mod cui;
mod helper;
mod interprocess;
mod session;
mod state;
mod tracing_helper;

use std::{ffi::c_void, path::PathBuf, sync::mpsc};

use junowen_lib::{
    hook_utils::{calc_th19_hash, WELL_KNOWN_VERSION_HASHES},
    Fn009fa0, Fn011560, Fn0d6e10, Fn1049e0, Fn10f720, FnOfHookAssembly, RenderingText, Selection,
    Th19,
};
use session::Session;
use state::State;
use tracing::warn;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, MAX_PATH},
        System::{
            Console::AllocConsole, LibraryLoader::GetModuleFileNameW,
            SystemServices::DLL_PROCESS_ATTACH,
        },
    },
};

use crate::interprocess::init_interprocess;

static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    old_on_input_players: Option<FnOfHookAssembly>,
    old_on_input_menu: Option<FnOfHookAssembly>,
    old_fn_from_0d7180_0008: Fn0d6e10,
    old_fn_from_11f870_034c: Fn1049e0,
    old_fn_from_1243f0_00f9: Fn011560,
    old_fn_from_1243f0_0320: Fn011560,
    old_fn_from_13f9d0_0345: Fn10f720,
    old_fn_from_13f9d0_0446: Fn009fa0,
    session_receiver: mpsc::Receiver<Session>,
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
    let props = props();
    state::on_input_players(state_mut(), props);

    if let Some(func) = props.old_on_input_players {
        func()
    }
}

extern "fastcall" fn on_input_menu() {
    state::on_input_menu(state_mut());

    let props = props();
    if let Some(func) = props.old_on_input_menu {
        func()
    }
}

extern "thiscall" fn on_render_text(this: *const c_void, arg: *const c_void) -> u32 {
    let ret = (props().old_fn_from_0d7180_0008)(this, arg);

    let state = state();
    if let Some(session) = state.session() {
        let th19 = state.th19();
        let mut text = RenderingText::default();
        text.set_text(format!("Delay: {}", session.delay()).as_bytes());
        text.x = (th19.screen_width().unwrap() * 1000 / 1280) as f32;
        text.y = (th19.screen_height().unwrap() * 940 / 960) as f32;
        text.color = 0xffffffff;
        text.font_type = 1;

        th19.render_text(this, &text);
    }

    ret
}

extern "fastcall" fn on_round_over() {
    (props().old_fn_from_11f870_034c)();

    state::on_round_over(state_mut());
}

fn is_online_vs(this: *const Selection, old: Fn011560) -> u8 {
    let ret = old(this);
    if state().session().is_some() {
        return 1;
    }
    ret
}

extern "thiscall" fn fn_from_1243f0_00f9(this: *const Selection) -> u8 {
    is_online_vs(this, props().old_fn_from_1243f0_00f9)
}

extern "thiscall" fn fn_from_1243f0_0320(this: *const Selection) -> u8 {
    is_online_vs(this, props().old_fn_from_1243f0_0320)
}

extern "fastcall" fn on_rewrite_controller_assignments() {
    state::on_rewrite_controller_assignments(props().old_fn_from_13f9d0_0345, state_mut());
}

extern "thiscall" fn on_loaded_game_settings(this: *const c_void, arg1: u32) -> u32 {
    state::on_loaded_game_settings(state_mut());

    (props().old_fn_from_13f9d0_0446)(this, arg1)
}

fn check_version() -> bool {
    let hash = calc_th19_hash();
    WELL_KNOWN_VERSION_HASHES
        .all_v100a()
        .iter()
        .any(|&valid_hash| valid_hash == &hash[..])
}

#[no_mangle]
pub extern "stdcall" fn DllMain(inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        if cfg!(debug_assertions) {
            let _ = unsafe { AllocConsole() };
            std::env::set_var("RUST_BACKTRACE", "1");
        }
        let dll_path = {
            let mut buf = [0u16; MAX_PATH as usize];
            if unsafe { GetModuleFileNameW(inst_dll, &mut buf) } == 0 {
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

        if !check_version() {
            warn!("version mismatch: {:?}", calc_th19_hash());
        }

        let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();

        let (old_on_input_players, apply_hook_on_input_players) =
            th19.hook_on_input_players(on_input_players);
        let (old_on_input_menu, apply_hook_on_input_menu) = th19.hook_on_input_menu(on_input_menu);
        let (old_fn_from_0d7180_0008, apply_hook_0d7180_0008) =
            th19.hook_0d7180_0008(on_render_text);
        let (old_fn_from_11f870_034c, apply_hook_11f870_034c) =
            th19.hook_11f870_034c(on_round_over);
        let (old_fn_from_1243f0_00f9, apply_hook_1243f0_00f9) =
            th19.hook_1243f0_00f9(fn_from_1243f0_00f9);
        let (old_fn_from_1243f0_0320, apply_hook_1243f0_0320) =
            th19.hook_1243f0_0320(fn_from_1243f0_0320);
        let (old_fn_from_13f9d0_0345, apply_hook_13f9d0_0345) =
            th19.hook_13f9d0_0345(on_rewrite_controller_assignments);
        let (old_fn_from_13f9d0_0446, apply_hook_13f9d0_0446) =
            th19.hook_13f9d0_0446(on_loaded_game_settings);

        let (session_sender, session_receiver) = mpsc::channel();
        init_interprocess(session_sender);
        unsafe {
            PROPS = Some(Props {
                old_on_input_players,
                old_on_input_menu,
                old_fn_from_0d7180_0008,
                old_fn_from_11f870_034c,
                old_fn_from_1243f0_00f9,
                old_fn_from_1243f0_0320,
                old_fn_from_13f9d0_0345,
                old_fn_from_13f9d0_0446,
                session_receiver,
            });
            STATE = Some(State::new(th19));
        }
        let th19 = &mut state_mut().th19_mut();
        apply_hook_on_input_players(th19);
        apply_hook_on_input_menu(th19);
        apply_hook_0d7180_0008(th19);
        apply_hook_11f870_034c(th19);
        apply_hook_1243f0_00f9(th19);
        apply_hook_1243f0_0320(th19);
        apply_hook_13f9d0_0345(th19);
        apply_hook_13f9d0_0446(th19);
    }
    true
}
