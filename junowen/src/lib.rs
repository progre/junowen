mod cui;
mod interprocess;
mod session;
mod state;
mod tracing_helper;

use std::{ffi::c_void, path::PathBuf, sync::mpsc};

use junowen_lib::{Fn009fa0, Fn1049e0, FnOfHookAssembly, Th19};
use session::Session;
use state::State;
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
    old_fn_from_11f870_034c: Fn1049e0,
    old_fn_from_13f9d0_0446: Fn009fa0,
    session_receiver: mpsc::Receiver<Session>,
}

fn props() -> &'static Props {
    unsafe { PROPS.as_ref().unwrap() }
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

extern "fastcall" fn on_round_over() {
    (props().old_fn_from_11f870_034c)();

    state::on_round_over(state_mut());
}

extern "thiscall" fn on_loaded_game_settings(this: *const c_void, arg1: u32) -> u32 {
    state::on_loaded_game_settings(state_mut());

    (props().old_fn_from_13f9d0_0446)(this, arg1)
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
        let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();

        let (old_on_input_players, apply_hook_on_input_players) =
            th19.hook_on_input_players(on_input_players);
        let (old_on_input_menu, apply_hook_on_input_menu) = th19.hook_on_input_menu(on_input_menu);
        let (old_fn_from_11f870_034c, apply_hook_11f870_034c) =
            th19.hook_11f870_034c(on_round_over);
        let (old_fn_from_13f9d0_0446, apply_hook_13f9d0_0446) =
            th19.hook_13f9d0_0446(on_loaded_game_settings);

        let (session_sender, session_receiver) = mpsc::channel();
        init_interprocess(session_sender);
        unsafe {
            PROPS = Some(Props {
                old_on_input_players,
                old_on_input_menu,
                old_fn_from_11f870_034c,
                old_fn_from_13f9d0_0446,
                session_receiver,
            });
            STATE = Some(State::new(th19));
        }
        let th19 = &mut state_mut().th19_mut();
        apply_hook_on_input_players(th19);
        apply_hook_on_input_menu(th19);
        apply_hook_11f870_034c(th19);
        apply_hook_13f9d0_0446(th19);
    }
    true
}
