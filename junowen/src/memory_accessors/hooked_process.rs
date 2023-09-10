use std::{ffi::c_void, mem::size_of};

use anyhow::{anyhow, Result};
use windows::{
    core::{Interface, HSTRING},
    Win32::{
        Graphics::Direct3D9::IDirect3DDevice9,
        System::{
            LibraryLoader::GetModuleHandleW,
            Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS},
            ProcessStatus::{GetModuleInformation, MODULEINFO},
            Threading::GetCurrentProcess,
        },
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

    pub fn as_direct_3d_device(&self, address: usize) -> Result<&'static IDirect3DDevice9> {
        unsafe {
            IDirect3DDevice9::from_raw_borrowed(
                &*((self.base_addr + address) as *const *mut c_void),
            )
        }
        .ok_or_else(|| anyhow!("IDirect3DDevice9::from_raw_borrowed failed"))
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

    pub fn hook_func(&self, addr: usize, target: usize) -> usize {
        let addr = self.base_addr + addr;

        let jump_base_addr = addr + 5;
        let jump_ref_addr = (addr + 1) as *mut i32;
        let old = (jump_base_addr as i64 + unsafe { *jump_ref_addr } as i64) as usize;
        unsafe { *jump_ref_addr = (target as i64 - jump_base_addr as i64) as i32 };
        old
    }
}
