use std::mem::size_of;

use anyhow::{anyhow, Result};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};

use crate::win_api_wrappers::SafeHandle;

fn find_process_id_in_snapshot(snapshot: SafeHandle, exe_file: &str) -> Option<u32> {
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
    if unsafe { Process32First(snapshot.0, &mut pe) }.is_err() {
        return None;
    }
    loop {
        let current = String::from_utf8_lossy(&pe.szExeFile);
        if current.contains(exe_file) {
            return Some(pe.th32ProcessID);
        }

        if unsafe { Process32Next(snapshot.0, &mut pe) }.is_err() {
            return None;
        }
    }
}

pub fn find_process_id(exe_file: &str) -> Result<u32> {
    let snapshot = SafeHandle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }?);

    let process_id = find_process_id_in_snapshot(snapshot, exe_file)
        .ok_or_else(|| anyhow!("Process not found"))?;

    Ok(process_id)
}
