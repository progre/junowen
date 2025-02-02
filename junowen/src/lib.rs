mod custom_direct_3d;
mod file;
mod helper;
mod junowen;
mod lobby;
mod session;
mod signaling;
mod state;
mod tracing_helper;

use std::{cell::OnceCell, mem::take, path::Path, ptr::null_mut, slice, sync::LazyLock};

use custom_direct_3d::CustomDirect3D9;
use junowen_lib::{
    Th19, Th19EventDispatcher,
    hook_utils::{WELL_KNOWN_VERSION_HASHES, calc_th19_hash, show_warn_dialog},
};
use windows::Win32::Graphics::Direct3D9::IDirect3D9;
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
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

async fn init(dll_path: &Path, direct_3d: Option<IDirect3D9>) -> Option<IDirect3D9> {
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
    let junowen_cell = unsafe { (&raw mut JUNOWEN).as_mut() }.unwrap();
    let junowen = junowen_cell.get_mut().unwrap();
    Th19EventDispatcher::init(th19, junowen);

    let direct_3d = direct_3d?;
    let junowen_cell = unsafe { (&raw mut JUNOWEN).as_mut() }.unwrap();
    let junowen = junowen_cell.get().unwrap();
    Some(IDirect3D9::from(CustomDirect3D9::new(direct_3d, junowen)))
}

fn launch_init(dll_path: &Path, direct_3d: Option<IDirect3D9>) -> Option<IDirect3D9> {
    TOKIO_RUNTIME.block_on(init(dll_path, direct_3d))
}

fn self_init() -> bool {
    let hash = calc_th19_hash();
    let dll_path = to_dll_path(unsafe { MODULE });
    if !check_version(&hash) {
        show_warn_dialog(&format!("Hash mismatch: {}", dll_path.to_string_lossy()));
        return false;
    }
    std::thread::spawn(move || {
        launch_init(&dll_path, None);
    });

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

/// # Safety
///
/// Pass a valid IDirect3D pointer.
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Initialize(direct_3d: *mut IDirect3D9) -> bool {
    let direct_3d_ref = unsafe { (direct_3d as *mut Option<IDirect3D9>).as_mut() }.unwrap();
    let direct_3d = take(direct_3d_ref);

    let dll_path = to_dll_path(unsafe { MODULE });
    let direct_3d = launch_init(&dll_path, direct_3d).unwrap();
    *direct_3d_ref = Some(direct_3d);
    true
}
