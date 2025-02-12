mod file;
mod helper;
mod junowen;
mod lobby;
mod session;
mod signaling;
mod state;
mod tracing_helper;

use std::{cell::OnceCell, path::Path, ptr::null_mut, slice, sync::LazyLock};

use junowen_lib::{
    Th19, Th19EventDispatcher,
    hook_utils::{WELL_KNOWN_VERSION_HASHES, calc_th19_hash, show_warn_dialog},
};
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    Graphics::Direct3D9::IDirect3D9,
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

use crate::{
    file::{SettingsRepo, to_dll_path, to_ini_file_path_log_dir_path_log_file_name},
    junowen::Junowen,
};

static TOKIO_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

static mut MODULE: HMODULE = HMODULE(null_mut());
static mut TH19: OnceCell<Th19> = OnceCell::new();
static mut JUNOWEN: OnceCell<Junowen> = OnceCell::new();

fn check_version(hash: &[u8]) -> bool {
    WELL_KNOWN_VERSION_HASHES
        .all_v110c()
        .iter()
        .any(|&valid_hash| valid_hash == hash)
}

async fn init(dll_path: &Path) {
    if cfg!(debug_assertions) {
        let _ = unsafe { AllocConsole() };
        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    }

    let dll_stem = dll_path.file_stem().unwrap().to_string_lossy();
    let (ini_file_path, module_dir, log_file_name) =
        to_ini_file_path_log_dir_path_log_file_name(&dll_stem);
    tracing_helper::init_tracing(&module_dir, &log_file_name, false);

    let th19_ptr = &raw mut TH19;
    let th19 = Th19::new_hooked_process("th19.exe").unwrap();
    let th19_cell = unsafe { th19_ptr.as_ref() }.unwrap();
    th19_cell.set(th19).map_err(|_| {}).unwrap();

    let th19 = unsafe { th19_ptr.as_mut() }.unwrap().get_mut().unwrap();
    let junowen = Junowen::new(SettingsRepo::new(ini_file_path), th19).await;
    let junowen_cell = unsafe { (&raw mut JUNOWEN).as_mut() }.unwrap();
    junowen_cell.set(junowen).map_err(|_| {}).unwrap();

    let th19 = unsafe { th19_ptr.as_mut() }.unwrap().get_mut().unwrap();
    let junowen = junowen_cell.get_mut().unwrap();
    Th19EventDispatcher::init(th19, junowen);
}

fn launch_init(dll_path: &Path) {
    TOKIO_RUNTIME.block_on(init(dll_path));
}

fn self_init() -> bool {
    let hash = calc_th19_hash();
    let dll_path = to_dll_path(unsafe { MODULE });
    if !check_version(&hash) {
        show_warn_dialog(&format!("Hash mismatch: {}", dll_path.to_string_lossy()));
        return false;
    }
    std::thread::spawn(move || launch_init(&dll_path));

    true
}

#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn CheckVersion(hash: *const u8, length: usize) -> bool {
    let hash = unsafe { slice::from_raw_parts(hash, length) };
    check_version(hash)
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Initialize(_direct_3d: *const IDirect3D9) -> bool {
    let dll_path = to_dll_path(unsafe { MODULE });
    launch_init(&dll_path);

    true
}
