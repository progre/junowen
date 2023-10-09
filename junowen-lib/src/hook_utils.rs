mod inject_dll;
mod load_library_w_addr;

use std::{fs::File, io::Read};

use sha3::digest::Digest; // using for Sha3_224::new()
use sha3::{digest::generic_array::GenericArray, Sha3_224};
use windows::{
    core::PCWSTR,
    Win32::{Foundation::MAX_PATH, System::LibraryLoader::GetModuleFileNameW},
};

pub use inject_dll::inject_dll;

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
    pub v100a: [u8; 28],
    pub v100a_steam: [u8; 28],
}

pub const WELL_KNOWN_VERSION_HASHES: WellKnownVersionHashes = WellKnownVersionHashes {
    v100a: [
        0xef, 0xf4, 0x38, 0x36, 0x51, 0xe5, 0xa2, 0x4b, 0x75, 0x11, 0xda, 0xa0, 0xd6, 0x44, 0x14,
        0x2c, 0x24, 0x39, 0xa8, 0x31, 0xe5, 0x36, 0x2d, 0xd9, 0xff, 0xbf, 0xf1, 0x89,
    ],
    v100a_steam: [
        0xaa, 0x4e, 0xf4, 0xe6, 0xfa, 0xe1, 0x23, 0xcb, 0xcb, 0xc1, 0xc2, 0xc2, 0x32, 0x46, 0x2d,
        0x5e, 0xfa, 0x6b, 0x21, 0x5d, 0x4a, 0x94, 0xf6, 0x4d, 0x62, 0xbc, 0xef, 0xcb,
    ],
};
