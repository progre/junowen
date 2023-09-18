use std::mem::transmute;

use anyhow::Result;
use windows::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY;

use crate::{memory_accessors::MemoryAccessor, BattleSettings};

pub fn value<T>(memory_accessor: &MemoryAccessor, addr: usize) -> &'static T {
    let MemoryAccessor::HookedProcess(memory_accessor) = memory_accessor else {
        panic!("Th19::object is only available for HookedProcess");
    };
    let p_obj = memory_accessor.raw_ptr(addr) as *const T;
    unsafe { p_obj.as_ref().unwrap() }
}
pub fn value_mut<T>(memory_accessor: &mut MemoryAccessor, addr: usize) -> &'static mut T {
    let MemoryAccessor::HookedProcess(memory_accessor) = memory_accessor else {
        panic!("Th19::object is only available for HookedProcess");
    };
    let p_obj = memory_accessor.raw_ptr(addr) as *mut T;
    unsafe { p_obj.as_mut().unwrap() }
}

pub fn pointer<T>(memory_accessor: &MemoryAccessor, addr: usize) -> Option<&'static T> {
    let MemoryAccessor::HookedProcess(memory_accessor) = memory_accessor else {
        panic!("Th19::object is only available for HookedProcess");
    };
    let p_p_obj = memory_accessor.raw_ptr(addr) as *const *const T;
    unsafe { (*p_p_obj).as_ref() }
}
pub fn pointer_mut<T>(memory_accessor: &mut MemoryAccessor, addr: usize) -> Option<&'static mut T> {
    let MemoryAccessor::HookedProcess(memory_accessor) = memory_accessor else {
        panic!("Th19::object is only available for HookedProcess");
    };
    let p_p_obj = memory_accessor.raw_ptr(addr) as *const *mut T;
    unsafe { (*p_p_obj).as_mut() }
}

pub fn hook_call(memory_accessor: &MemoryAccessor, addr: usize, target: usize) -> Result<usize> {
    let MemoryAccessor::HookedProcess(memory_accessor) = memory_accessor else {
        panic!("Th19::hook_call is only available for HookedProcess");
    };
    let old = memory_accessor.virtual_protect(addr, 5, PAGE_EXECUTE_WRITECOPY)?;
    let original = memory_accessor.hook_call(addr, target);
    memory_accessor.virtual_protect(addr, 5, old)?;
    Ok(original)
}

pub fn battle_settings_from(
    memory_accessor: &MemoryAccessor,
    addr: usize,
) -> Result<BattleSettings> {
    let mut buffer = [0u8; 12];
    memory_accessor.read(addr, &mut buffer)?;
    Ok(unsafe { transmute(buffer) })
}
pub fn put_battle_settings_to(
    memory_accessor: &mut MemoryAccessor,
    addr: usize,
    battle_settings: &BattleSettings,
) -> Result<()> {
    let buffer: &[u8; 12] = unsafe { transmute(battle_settings) };
    memory_accessor.write(addr, buffer)
}
