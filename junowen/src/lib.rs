mod interprocess;

use std::sync::mpsc;

use windows::Win32::{
    Foundation::HINSTANCE,
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

use crate::interprocess::init_interprocess;

#[no_mangle]
pub extern "stdcall" fn DllMain(_inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        if cfg!(debug_assertions) {
            let _ = unsafe { AllocConsole() };
            std::env::set_var("RUST_BACKTRACE", "1");
        }
        let (session_sender, _session_receiver) = mpsc::channel();
        init_interprocess(session_sender);
    }
    true
}
