use std::{
    ffi::c_void,
    mem::{size_of, transmute},
    ptr::{copy_nonoverlapping, read_unaligned, write_unaligned},
};

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

pub type FnOfHookAssembly = extern "fastcall" fn();

const MAX_HOOK_ASSEMBLY_CAPACITY: usize = 16;

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

unsafe fn assemble_call_and_manage_register(mut addr: *mut u8, target: usize) {
    *(addr as *mut [u8; 3]) = [
        0x50, // push eax
        0x51, // push ecx
        0x52, // push edx
    ];
    addr = addr.wrapping_add(3);

    *addr = 0xe8;
    assemble_jmp_target(addr, target);
    addr = addr.wrapping_add(5);

    *(addr as *mut [u8; 3]) = [
        0x5a, // pop edx
        0x59, // pop ecx
        0x58, // pop eax
    ];
}

unsafe fn assemble_jmp_target(addr: *mut u8, target: usize) -> usize {
    let jump_base_addr = addr.wrapping_add(5) as i64;
    let p_jump_target = addr.wrapping_add(1) as *mut i32;
    let old_value = read_unaligned(p_jump_target);
    write_unaligned(p_jump_target, (target as i64 - jump_base_addr) as i32);
    (jump_base_addr + old_value as i64) as usize
}

fn jmp_target(addr: *const u8) -> usize {
    let jump_base_addr = addr.wrapping_add(5) as i64;
    let p_jump_target = addr.wrapping_add(1) as *const i32;
    let value = unsafe { *p_jump_target };
    (jump_base_addr + value as i64) as usize
}

macro_rules! p_call_at_jmp_dst_impl {
    ($addr:ident) => {
        (0..MAX_HOOK_ASSEMBLY_CAPACITY)
            .map(|i| $addr.wrapping_add(i))
            .find(|&a| (unsafe { *a }) == 0xe8)
            .unwrap()
    };
}
fn p_call_at_jmp_dst(addr: *const u8) -> *const u8 {
    p_call_at_jmp_dst_impl!(addr)
}
fn p_call_at_jmp_dst_mut(addr: *mut u8) -> *mut u8 {
    p_call_at_jmp_dst_impl!(addr)
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

    pub fn write(&mut self, addr: usize, buffer: &[u8]) {
        unsafe { ((self.base_addr + addr) as *mut u8).copy_from(buffer.as_ptr(), buffer.len()) };
    }

    pub fn raw_ptr(&self, addr: usize) -> *const c_void {
        (self.base_addr + addr) as *const c_void
    }

    pub fn virtual_protect(
        &mut self,
        addr: usize,
        size: usize,
        protect: PAGE_PROTECTION_FLAGS,
    ) -> Result<PAGE_PROTECTION_FLAGS> {
        let mut old: PAGE_PROTECTION_FLAGS = Default::default();
        unsafe { VirtualProtect((self.base_addr + addr) as _, size, protect, &mut old) }?;
        Ok(old)
    }

    pub fn virtual_protect_global(
        &mut self,
        addr: usize,
        size: usize,
        protect: PAGE_PROTECTION_FLAGS,
    ) -> Result<PAGE_PROTECTION_FLAGS> {
        let mut old: PAGE_PROTECTION_FLAGS = Default::default();
        unsafe { VirtualProtect(addr as _, size, protect, &mut old) }?;
        Ok(old)
    }

    pub fn hook_call(&mut self, addr: usize, target: usize) -> usize {
        unsafe { assemble_jmp_target((self.base_addr + addr) as *mut u8, target) }
    }

    pub fn current_callback_of_hook_call(&self, addr: usize) -> usize {
        jmp_target((self.base_addr + addr) as *const u8)
    }

    /// Create a call to a dummy function where there is no call,
    /// copy the original instruction to the dummy function,
    /// and call the target function from the dummy function.
    pub fn hook_assembly(
        &mut self,
        addr: usize,
        capacity: usize,
        dummy_func: extern "fastcall" fn(),
        target_func: extern "fastcall" fn(),
    ) -> Option<extern "fastcall" fn()> {
        debug_assert!(
            (5..MAX_HOOK_ASSEMBLY_CAPACITY).contains(&capacity),
            "capacity must be 9..{}",
            MAX_HOOK_ASSEMBLY_CAPACITY
        );
        let mut addr = (self.base_addr + addr) as *mut u8;
        assert!(
            (0..capacity)
                .filter(|&i| i != 2)
                .map(|i| addr.wrapping_add(i))
                .all(|ptr| unsafe { *ptr } != 0xe8),
            "hook target must not have call instruction"
        );

        let already_hooked = unsafe { *addr } == 0xe8;
        if already_hooked {
            let p_dummy_func = jmp_target(addr) as *mut u8;

            let p_call = p_call_at_jmp_dst_mut(p_dummy_func);
            return Some(unsafe {
                transmute::<usize, extern "fastcall" fn()>(assemble_jmp_target(
                    p_call,
                    target_func as usize,
                ))
            });
        }

        let p_dummy_func = dummy_func as *mut u8;
        let has_dummy_func_machine_code_capacity = (0..capacity + 5 + 6)
            .map(|i| p_dummy_func.wrapping_add(i))
            .all(|dummy_func_nop_addr| unsafe { *dummy_func_nop_addr } == 0x90);
        debug_assert!(
            has_dummy_func_machine_code_capacity,
            "dummy_func must have enough machine code capacity"
        );
        unsafe { copy_nonoverlapping(addr, p_dummy_func, capacity) };

        unsafe {
            assemble_call_and_manage_register(p_dummy_func.wrapping_add(capacity), target_func as _)
        };

        unsafe {
            *addr = 0xe8;
            assemble_jmp_target(addr, dummy_func as usize);
        }
        addr = addr.wrapping_add(5);

        for i in 0..(capacity - 5) {
            unsafe { *addr.wrapping_add(i) = 0x90 }; // nop
        }

        None
    }

    pub fn current_callback_of_hook_assembly(&self, addr: usize) -> Option<FnOfHookAssembly> {
        let addr = (self.base_addr + addr) as *const u8;
        let already_hooked = unsafe { *addr.wrapping_add(2) } == 0xe8;
        if !already_hooked {
            return None;
        }
        let p_dummy_func = jmp_target(addr) as *const u8;

        let p_call = p_call_at_jmp_dst(p_dummy_func);
        Some(unsafe { transmute::<usize, FnOfHookAssembly>(jmp_target(p_call)) })
    }
}
