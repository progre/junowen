use std::{ffi::c_void, mem::size_of};

use anyhow::Result;
use windows::{
    core::HSTRING,
    Win32::System::{
        LibraryLoader::GetModuleHandleW,
        Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS},
        ProcessStatus::{GetModuleInformation, MODULEINFO},
        Threading::GetCurrentProcess,
    },
};

fn module_base_addr(module_name: &str) -> Result<usize> {
    let module = unsafe { GetModuleHandleW(&HSTRING::from(module_name)) }?;
    let mut module_info: MODULEINFO = Default::default();
    unsafe {
        GetModuleInformation(
            GetCurrentProcess(),
            module,
            &mut module_info,
            size_of::<MODULEINFO>() as u32,
        )
    }?;
    Ok(module_info.lpBaseOfDll as usize)
}

pub struct HookedProcess {
    base_addr: usize,
}

impl HookedProcess {
    pub fn new(exe_file: &str) -> Result<Self> {
        Ok(Self {
            base_addr: module_base_addr(exe_file)?,
        })
    }

    pub fn read(&self, addr: usize, buffer: &mut [u8]) {
        unsafe { ((self.base_addr + addr) as *mut u8).copy_to(buffer.as_mut_ptr(), buffer.len()) };
    }

    pub fn write(&self, addr: usize, buffer: &[u8]) {
        unsafe { ((self.base_addr + addr) as *mut u8).copy_from(buffer.as_ptr(), buffer.len()) };
    }

    pub fn raw_ptr(&self, addr: usize) -> *const c_void {
        (self.base_addr + addr) as *const c_void
    }

    pub fn virtual_protect(
        &self,
        addr: usize,
        size: usize,
        protect: PAGE_PROTECTION_FLAGS,
    ) -> Result<PAGE_PROTECTION_FLAGS> {
        let mut old: PAGE_PROTECTION_FLAGS = Default::default();
        unsafe { VirtualProtect((self.base_addr + addr) as _, size, protect, &mut old) }?;
        Ok(old)
    }

    pub fn hook_call(&self, addr: usize, target: usize) -> usize {
        let addr = self.base_addr + addr;

        let jump_base_addr = addr + 5;
        let jump_ref_addr = (addr + 1) as *mut i32;
        let old = (jump_base_addr as i64 + unsafe { *jump_ref_addr } as i64) as usize;
        unsafe { *jump_ref_addr = (target as i64 - jump_base_addr as i64) as i32 };
        old
    }

    pub fn hook_assembly(&self, addr: usize, capacity: usize, target: usize) -> Option<usize> {
        if capacity < 9 {
            panic!("capacity must be at least 9");
        }
        let mut addr = (self.base_addr + addr) as *mut u8;
        unsafe { *addr = 0x51 }; // push ecx
        addr = addr.wrapping_add(1);
        unsafe { *addr = 0x52 }; // push edx
        addr = addr.wrapping_add(1);

        // call target
        let jump_base_addr = addr.wrapping_add(5) as u32;
        let jump_ref_addr = addr.wrapping_add(1) as *mut i32;
        let mut old = None;
        if unsafe { *addr } == 0xe8 {
            old = Some((jump_base_addr as i64 + unsafe { *jump_ref_addr } as i64) as usize);
        }
        unsafe { *addr = 0xe8 };
        unsafe { *jump_ref_addr = (target as i64 - jump_base_addr as i64) as i32 };
        addr = addr.wrapping_add(5);

        unsafe { *addr = 0x5a }; // pop edx
        addr = addr.wrapping_add(1);
        unsafe { *addr = 0x59 }; // pop ecx
        addr = addr.wrapping_add(1);

        for i in 0..(capacity - 9) {
            unsafe { *addr.wrapping_add(i) = 0x90 }; // nop
        }

        old
    }
}
