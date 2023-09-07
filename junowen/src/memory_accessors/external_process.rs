use std::{ffi::c_void, mem::size_of};

use anyhow::{anyhow, bail, Result};
use windows::Win32::{
    Foundation::{CloseHandle, FALSE, HANDLE, HMODULE, MAX_PATH},
    System::{
        Diagnostics::{
            Debug::{ReadProcessMemory, WriteProcessMemory},
            ToolHelp::{
                CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
                TH32CS_SNAPPROCESS,
            },
        },
        ProcessStatus::{EnumProcessModules, GetModuleBaseNameA},
        Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ,
            PROCESS_VM_WRITE,
        },
    },
};

use super::MemoryAccessor;

fn find_process_id_in_snapshot(snapshot: HANDLE, exe_file: &str) -> Option<u32> {
    let mut pe = PROCESSENTRY32 {
        dwSize: size_of::<PROCESSENTRY32>() as u32,
        cntUsage: 0,
        th32ProcessID: 0,
        th32DefaultHeapID: 0,
        th32ModuleID: 0,
        cntThreads: 0,
        th32ParentProcessID: 0,
        pcPriClassBase: 0,
        dwFlags: 0,
        szExeFile: [0; 260],
    };
    if unsafe { Process32First(snapshot, &mut pe) }.is_err() {
        return None;
    }
    loop {
        let current = String::from_utf8_lossy(&pe.szExeFile);
        if current.contains(exe_file) {
            return Some(pe.th32ProcessID);
        }

        if unsafe { Process32Next(snapshot, &mut pe) }.is_err() {
            return None;
        }
    }
}

pub fn find_process_id(exe_file: &str) -> Result<u32> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }?;

    let process_id = find_process_id_in_snapshot(snapshot, exe_file)
        .ok_or_else(|| anyhow!("Process not found"))?;

    unsafe { CloseHandle(snapshot) }?;
    Ok(process_id)
}

fn find_base_module(process: HANDLE) -> Result<HMODULE> {
    let mut modules = [HMODULE::default(); 1024];
    let mut cb_needed = 0;
    unsafe {
        EnumProcessModules(
            process,
            modules.as_mut_ptr(),
            size_of::<[HMODULE; 1024]>() as u32,
            &mut cb_needed,
        )
    }?;
    let num_modules = cb_needed as usize / size_of::<HMODULE>();

    modules[0..num_modules]
        .iter()
        .filter(|&&module| {
            let mut base_name = [0u8; MAX_PATH as usize];
            let len = unsafe { GetModuleBaseNameA(process, module, &mut base_name) };
            len > 0 && &String::from_utf8_lossy(&base_name[0..len as usize]) == "th19.exe"
        })
        .copied()
        .next()
        .ok_or(anyhow!("module not found"))
}

pub struct ExternalProcess {
    process: HANDLE,
    base_module: HMODULE,
}

impl ExternalProcess {
    pub fn new() -> Result<Self> {
        let process_id = find_process_id("th19.exe")?;
        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION
                    | PROCESS_VM_OPERATION
                    | PROCESS_VM_READ
                    | PROCESS_VM_WRITE,
                FALSE,
                process_id,
            )
        }?;
        let base_module = find_base_module(process)?;

        Ok(Self {
            process,
            base_module,
        })
    }
}

impl MemoryAccessor for ExternalProcess {
    fn read(&self, address: usize, buffer: &mut [u8]) -> Result<()> {
        let mut number_of_bytes_read: usize = 0;
        unsafe {
            ReadProcessMemory(
                self.process,
                (self.base_module.0 as usize + address) as *const c_void,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                Some(&mut number_of_bytes_read),
            )
        }?;
        if number_of_bytes_read != buffer.len() {
            bail!("ReadProcessMemory failed");
        }
        Ok(())
    }

    fn write(&self, address: usize, buffer: &[u8]) -> Result<()> {
        let mut number_of_bytes_written: usize = 0;
        unsafe {
            WriteProcessMemory(
                self.process,
                (self.base_module.0 as usize + address) as *const c_void,
                buffer.as_ptr() as *const c_void,
                buffer.len(),
                Some(&mut number_of_bytes_written),
            )
        }?;
        if number_of_bytes_written != buffer.len() {
            bail!("WriteProcessMemory failed");
        }
        Ok(())
    }
}

impl Drop for ExternalProcess {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.process) }.unwrap();
    }
}
