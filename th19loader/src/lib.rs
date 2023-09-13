use std::{env::current_dir, fs::File, io::Read, mem::transmute};

use sha3::digest::Digest; // Sha3_224::new() で使用

use sha3::{digest::generic_array::GenericArray, Sha3_224};
use windows::{
    core::{s, HSTRING, PCWSTR},
    Win32::{
        Foundation::{FreeLibrary, HINSTANCE, HMODULE, HWND, MAX_PATH},
        Graphics::Direct3D9::IDirect3D9,
        System::{
            Console::AllocConsole,
            LibraryLoader::{GetModuleFileNameW, GetProcAddress, LoadLibraryW},
            SystemInformation::GetSystemDirectoryW,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
        UI::WindowsAndMessaging::{MessageBoxW, MB_ICONWARNING, MB_OK},
    },
};

static mut ORIGINAL_DIRECT_3D_CREATE_9: usize = 0;
static mut ORIGINAL_MODULE: HMODULE = HMODULE(0);

fn load_library(dll_name: &str) -> HMODULE {
    let system_directory = unsafe {
        let mut buf = [0u16; MAX_PATH as usize];
        GetSystemDirectoryW(Some(&mut buf));
        PCWSTR::from_raw(buf.as_ptr()).to_string().unwrap()
    };
    let dll_path = format!("{}\\{}", system_directory, dll_name);
    let dll_instance = unsafe { LoadLibraryW(&HSTRING::from(dll_path)) }.unwrap();
    if dll_instance.is_invalid() {
        panic!();
    }
    dll_instance
}

fn show_warn_dialog(msg: &str) {
    unsafe {
        MessageBoxW(
            HWND::default(),
            &HSTRING::from(msg),
            &HSTRING::from(env!("CARGO_PKG_NAME")),
            MB_ICONWARNING | MB_OK,
        )
    };
}

fn calc_th19_hash() -> Vec<u8> {
    let mut buf = [0u16; MAX_PATH as usize];
    if unsafe { GetModuleFileNameW(None, &mut buf) } == 0 {
        panic!();
    }
    let exe_file_path = unsafe { PCWSTR::from_raw(buf.as_ptr()).to_string() }.unwrap();
    let mut file = File::open(exe_file_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let mut hasher: Sha3_224 = Sha3_224::new();
    hasher.update(&buffer);
    let hash: GenericArray<_, _> = hasher.finalize();
    hash.to_vec()
}

fn hook(direct_3d: *const IDirect3D9) {
    let hash = calc_th19_hash();

    let directory = current_dir().unwrap().join("modules");
    if !directory.is_dir() {
        return;
    }
    directory
        .read_dir()
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_ascii_uppercase()
                .starts_with("TH19_")
                && path.extension().unwrap().to_ascii_uppercase() == "DLL"
        })
        .for_each(|path| {
            let module =
                unsafe { LoadLibraryW(&HSTRING::from(path.to_string_lossy().as_ref())) }.unwrap();
            if module.is_invalid() {
                show_warn_dialog(&format!("Failed to load {}", path.to_string_lossy()));
                return;
            }

            let Some(check_version) = (unsafe { GetProcAddress(module, s!("CheckVersion")) })
            else {
                show_warn_dialog(&format!(
                    "Failed to get CheckHash from {}",
                    path.to_string_lossy()
                ));
                unsafe { FreeLibrary(module) }.unwrap();
                return;
            };
            let check_version: extern "C" fn(hash: *const u8, length: usize) -> bool =
                unsafe { transmute(check_version) };
            if !check_version(hash.as_ptr(), hash.len()) {
                show_warn_dialog(&format!("Hash mismatch: {}", path.to_string_lossy()));
                unsafe { FreeLibrary(module) }.unwrap();
                return;
            }

            let Some(initialize_addr) = (unsafe { GetProcAddress(module, s!("Initialize")) })
            else {
                show_warn_dialog(&format!(
                    "Failed to get Initialize from {}",
                    path.to_string_lossy()
                ));
                unsafe { FreeLibrary(module) }.unwrap();
                return;
            };
            let initialize: extern "C" fn(direct_3d: *const IDirect3D9) -> bool =
                unsafe { transmute(initialize_addr) };
            if !initialize(direct_3d) {
                show_warn_dialog(&format!("Failed to initialize {}", path.to_string_lossy()));
                unsafe { FreeLibrary(module) }.unwrap();
            }
        });
}

#[no_mangle]
pub extern "stdcall" fn DllMain(_inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    match reason {
        DLL_PROCESS_ATTACH => {
            if cfg!(debug_assertions) {
                let _ = unsafe { AllocConsole() };
            }

            let dll_instance = load_library("d3d9.dll");
            let func = unsafe { GetProcAddress(dll_instance, s!("Direct3DCreate9")) }.unwrap();
            unsafe {
                ORIGINAL_MODULE = dll_instance;
                ORIGINAL_DIRECT_3D_CREATE_9 = func as usize;
            }
        }
        DLL_PROCESS_DETACH => unsafe {
            if !ORIGINAL_MODULE.is_invalid() {
                FreeLibrary(ORIGINAL_MODULE).unwrap();
            }
        },
        _ => {}
    }
    true
}

#[no_mangle]
extern "stdcall" fn Direct3DCreate9(sdkversion: u32) -> *const IDirect3D9 {
    type Func = extern "stdcall" fn(sdkversion: u32) -> *const IDirect3D9;
    let func: Func = unsafe { transmute(ORIGINAL_DIRECT_3D_CREATE_9) };
    let direct_3d = func(sdkversion);
    hook(direct_3d);
    direct_3d
}
