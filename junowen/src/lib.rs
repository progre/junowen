mod file;
mod helper;
mod in_game_lobby;
mod session;
mod signaling;
mod state;
mod th19_event_dispatcher;
mod tracing_helper;

use std::{ffi::c_void, ptr::null, slice, sync::LazyLock};

use junowen_lib::{
    hook_utils::{calc_th19_hash, show_warn_dialog, WELL_KNOWN_VERSION_HASHES},
    Th19,
};
use th19_event_dispatcher::Th19EventDispatcher;
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    Graphics::Direct3D9::IDirect3D9,
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

use crate::{
    file::{
        move_old_log_to_new_path, to_dll_path, to_ini_file_path_log_dir_path_log_file_name,
        SettingsRepo,
    },
    state::State,
};

static TOKIO_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

static mut MODULE: HMODULE = HMODULE(null::<c_void>() as *mut _);
static mut STATE: Option<State> = None;

fn state() -> &'static State {
    let state = &raw const STATE;
    unsafe { state.as_ref() }.unwrap().as_ref().unwrap()
}
fn state_mut() -> &'static mut State {
    let state = &raw mut STATE;
    unsafe { state.as_mut() }.unwrap().as_mut().unwrap()
}

fn check_version(hash: &[u8]) -> bool {
    WELL_KNOWN_VERSION_HASHES
        .all_v110c()
        .iter()
        .any(|&valid_hash| valid_hash == hash)
}

async fn init(dll_stem: &str, old_log_dir_path: Option<&str>) {
    if cfg!(debug_assertions) {
        let _ = unsafe { AllocConsole() };
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let (ini_file_path, module_dir, log_file_name) =
        to_ini_file_path_log_dir_path_log_file_name(dll_stem);
    tracing_helper::init_tracing(&module_dir, &log_file_name, false);
    if let Some(old_log_dir_path) = old_log_dir_path {
        let old_log_path = format!("{}/{}", old_log_dir_path, log_file_name);
        move_old_log_to_new_path(&old_log_path, &module_dir, &log_file_name).await;
    }

    let th19 = Th19::new_hooked_process("th19.exe").unwrap();

    unsafe {
        STATE = Some(State::new(SettingsRepo::new(ini_file_path), th19).await);
    }

    Th19EventDispatcher::init(state_mut().th19_mut());
}

fn launch_init(dll_stem: &str, old_log_dir_path: Option<&str>) {
    TOKIO_RUNTIME.block_on(init(dll_stem, old_log_dir_path));
}

fn self_init() -> bool {
    let hash = calc_th19_hash();
    let dll_path = to_dll_path(unsafe { MODULE });
    if !check_version(&hash) {
        show_warn_dialog(&format!("Hash mismatch: {}", dll_path.to_string_lossy()));
        return false;
    }
    let dll_stem = dll_path.file_stem().unwrap().to_string_lossy().to_string();
    std::thread::spawn(move || launch_init(&dll_stem, None));

    true
}

#[no_mangle]
pub extern "stdcall" fn DllMain(inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        unsafe { MODULE = inst_dll.into() };
        if cfg!(feature = "simple-dll-injection") && !self_init() {
            return false;
        }
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
    let dll_path = to_dll_path(unsafe { MODULE });
    let dll_stem = dll_path.file_stem().unwrap().to_string_lossy();
    let old_log_dir_path = dll_path.parent().unwrap().to_string_lossy();

    launch_init(&dll_stem, Some(&old_log_dir_path));

    true
}
