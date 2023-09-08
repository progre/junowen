mod load_library_w_addr;

use std::{
    mem::{size_of_val, transmute},
    os::raw::c_void,
    path::Path,
};

use anyhow::Result;
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::FALSE,
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            Memory::{VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE},
            Threading::{CreateRemoteThread, OpenProcess, WaitForSingleObject, PROCESS_ALL_ACCESS},
        },
    },
};

use crate::{find_process_id::find_process_id, win_api_wrappers::SafeHandle};

use load_library_w_addr::load_library_w_addr;

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

pub fn inject_dll(exe_file: &str, dll_path: &Path) -> Result<()> {
    let process_id = find_process_id(exe_file)?;
    let process = SafeHandle(unsafe { OpenProcess(PROCESS_ALL_ACCESS, FALSE, process_id) }?);
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
    }?;
    let thread = SafeHandle(unsafe {
        CreateRemoteThread(
            process.0,
            None,
            0,
            transmute(load_library_w_addr(process_id)?),
            Some(remote_dll_path_wstr.addr),
            0,
            None,
        )
    }?);

    unsafe { WaitForSingleObject(thread.0, u32::MAX) }; // wait DllMain

    Ok(())
}
