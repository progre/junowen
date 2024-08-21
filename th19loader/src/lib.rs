use std::{env::current_dir, ffi::c_void, mem::transmute, ptr::null};

use junowen_lib::hook_utils::{calc_th19_hash, show_warn_dialog};

use windows::{
    core::{s, HSTRING, PCWSTR},
    Win32::{
        Foundation::{FreeLibrary, HINSTANCE, HMODULE, MAX_PATH},
        Graphics::Direct3D9::IDirect3D9,
        System::{
            Console::AllocConsole,
            LibraryLoader::{GetProcAddress, LoadLibraryW},
            SystemInformation::GetSystemDirectoryW,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
};

static mut ORIGINAL_DIRECT_3D_CREATE_9: usize = 0;
static mut ORIGINAL_MODULE: HMODULE = HMODULE(null::<c_void>() as *mut _);

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

fn hook(direct_3d: *const IDirect3D9) {
    let hash = calc_th19_hash();
    println!("{:x?}", hash);

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
            let path = path.to_string_lossy();
            let module = unsafe { LoadLibraryW(&HSTRING::from(path.as_ref())) }.unwrap();
            if module.is_invalid() {
                show_warn_dialog(&format!("Failed to load {}", path));
                return;
            }

            let Some(check_version) = (unsafe { GetProcAddress(module, s!("CheckVersion")) })
            else {
                show_warn_dialog(&format!("Failed to get CheckVersion from {}", path));
                unsafe { FreeLibrary(module) }.unwrap();
                return;
            };
            let check_version: extern "C" fn(hash: *const u8, length: usize) -> bool =
                unsafe { transmute(check_version) };
            if !check_version(hash.as_ptr(), hash.len()) {
                show_warn_dialog(&format!("Hash mismatch: {}", path));
                unsafe { FreeLibrary(module) }.unwrap();
                return;
            }

            let Some(initialize_addr) = (unsafe { GetProcAddress(module, s!("Initialize")) })
            else {
                show_warn_dialog(&format!("Failed to get Initialize from {}", path));
                unsafe { FreeLibrary(module) }.unwrap();
                return;
            };
            let initialize: extern "C" fn(direct_3d: *const IDirect3D9) -> bool =
                unsafe { transmute(initialize_addr) };
            if !initialize(direct_3d) {
                show_warn_dialog(&format!("Failed to initialize {}", path));
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
