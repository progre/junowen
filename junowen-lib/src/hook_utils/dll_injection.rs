use std::{
    mem::{size_of_val, transmute},
    os::raw::c_void,
    path::Path,
};

use anyhow::{Error, Result};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::FALSE,
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            Memory::{VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE},
            Threading::{
                CreateRemoteThread, OpenProcess, WaitForSingleObject, LPTHREAD_START_ROUTINE,
                PROCESS_ALL_ACCESS,
            },
        },
    },
};

use crate::{find_process_id::find_process_id, win_api_wrappers::SafeHandle};

use super::load_library_w_addr::load_library_w_addr;

struct VirtualAllocatedMem<'a> {
    process: &'a SafeHandle,
    pub addr: *mut c_void,
}

impl<'a> VirtualAllocatedMem<'a> {
    pub fn new(process: &'a SafeHandle, size: usize) -> Self {
        Self {
            process,
            addr: unsafe { VirtualAllocEx(process.0, None, size, MEM_COMMIT, PAGE_READWRITE) },
        }
    }
}

impl<'a> Drop for VirtualAllocatedMem<'a> {
    fn drop(&mut self) {
        unsafe { VirtualFreeEx(self.process.0, self.addr, 0, MEM_RELEASE) }.unwrap();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DllInjectionError {
    #[error("DLL not found")]
    DllNotFound,
    #[error("Process not found: {}", .0)]
    ProcessNotFound(Error),
}

pub fn do_dll_injection(exe_file: &str, dll_path: &Path) -> Result<(), DllInjectionError> {
    if !dll_path.exists() {
        return Err(DllInjectionError::DllNotFound);
    }
    let process_id = find_process_id(exe_file).map_err(DllInjectionError::ProcessNotFound)?;
    let process = SafeHandle(
        unsafe { OpenProcess(PROCESS_ALL_ACCESS, FALSE, process_id) }
            .map_err(|err| DllInjectionError::ProcessNotFound(Error::new(err)))?,
    );
    let dll_path_hstr = HSTRING::from(dll_path);
    let dll_path_hstr_size = size_of_val(dll_path_hstr.as_wide());
    let remote_dll_path_wstr = VirtualAllocatedMem::new(&process, dll_path_hstr_size);

    unsafe {
        WriteProcessMemory(
            process.0,
            remote_dll_path_wstr.addr,
            dll_path_hstr.as_ptr() as _,
            dll_path_hstr_size,
            None,
        )
    }
    .unwrap();
    let load_library_w_addr = load_library_w_addr(process_id).unwrap();
    let thread = SafeHandle(
        unsafe {
            CreateRemoteThread(
                process.0,
                None,
                0,
                transmute::<usize, LPTHREAD_START_ROUTINE>(load_library_w_addr),
                Some(remote_dll_path_wstr.addr),
                0,
                None,
            )
        }
        .unwrap(),
    );

    unsafe { WaitForSingleObject(thread.0, u32::MAX) }; // wait thread

    Ok(())
}
