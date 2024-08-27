mod dll_injection;
mod load_library_w_addr;

use std::{fs::File, io::Read};

use hex_literal::hex;
use sha3::digest::Digest; // using for Sha3_224::new()
use sha3::{digest::generic_array::GenericArray, Sha3_224};
use windows::{
    core::{HSTRING, PCWSTR},
    Win32::{
        Foundation::{HWND, MAX_PATH},
        System::LibraryLoader::GetModuleFileNameW,
        UI::WindowsAndMessaging::{MessageBoxW, MB_ICONWARNING, MB_OK},
    },
};

pub use dll_injection::{do_dll_injection, DllInjectionError};

pub fn show_warn_dialog(msg: &str) {
    unsafe {
        MessageBoxW(
            HWND::default(),
            &HSTRING::from(msg),
            &HSTRING::from(env!("CARGO_PKG_NAME")),
            MB_ICONWARNING | MB_OK,
        )
    };
}

pub fn calc_th19_hash() -> Vec<u8> {
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

pub struct WellKnownVersionHashes {
    pub v110c: [u8; 28],
    pub v110c_steam: [u8; 28],
}

impl WellKnownVersionHashes {
    pub fn all_v110c(&self) -> [&[u8; 28]; 2] {
        [&self.v110c, &self.v110c_steam]
    }
}

pub const WELL_KNOWN_VERSION_HASHES: WellKnownVersionHashes = WellKnownVersionHashes {
    v110c: hex!("f7cfd5dc38a4cab6efd91646264b09f21cd79409d568f23b7cbfd359"),
    v110c_steam: hex!("a2bbb4ff6c7ee5bd1126b536416762f2bea3b83ebf351f24cb66af64"),
};
