mod find_process_id;
pub mod inject_dll;
pub mod memory_accessors;
mod win_api_wrappers;

use std::{ffi::c_void, mem::size_of};

use anyhow::{anyhow, Result};
use memory_accessors::{ExternalProcess, HookedProcess, MemoryAccessor};
use windows::{
    core::Interface,
    Win32::{Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY},
};

pub struct Th19 {
    memory_accessor: MemoryAccessor,
}

macro_rules! u16_prop {
    ($addr:expr, $getter:ident) => {
        pub fn $getter(&self) -> Result<u16> {
            self.memory_accessor.read_u16($addr)
        }
    };

    ($addr:expr, $getter:ident, $setter:ident) => {
        u16_prop!($addr, $getter);
        pub fn $setter(&self, value: u16) -> Result<()> {
            self.memory_accessor.write_u16($addr, value)
        }
    };
}

#[repr(C)] // 0x3d4
pub struct InputDevice {
    pub input: u32,
    _unknown2: [u8; 0x3d0],
}

#[repr(C)]
pub struct Input {
    pub _unknown1: [u8; 0x30],
    pub input_device_array: [InputDevice; 3 + 9],
    _unknown2: u32,
    pub p1_input_idx: u32,
    pub p2_input_idx: u32,
    // unknown continues...
}

impl Th19 {
    pub fn new_external_process(exe_file: &str) -> Result<Self> {
        Ok(Self {
            memory_accessor: MemoryAccessor::ExternalProcess(ExternalProcess::new(exe_file)?),
        })
    }

    pub fn new_hooked_process(exe_file: &str) -> Result<Self> {
        Ok(Self {
            memory_accessor: MemoryAccessor::HookedProcess(HookedProcess::new(exe_file)?),
        })
    }

    pub fn hook_0a96b5(&self, target: usize) -> Result<usize> {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hook_0a96b5 is only available for HookedProcess");
        };
        let old = memory_accessor.virtual_protect(0x0a96b5, 5, PAGE_EXECUTE_WRITECOPY)?;
        let original = memory_accessor.hook_call(0x0a96b5, target);
        memory_accessor.virtual_protect(0x0a96b5, 5, old)?;
        Ok(original)
    }

    pub fn hook_0abb2b(&self, target: usize) -> Result<()> {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hook_0abb2b is only available for HookedProcess");
        };
        let old = memory_accessor.virtual_protect(0x0abb2b, 14, PAGE_EXECUTE_WRITECOPY)?;
        memory_accessor.hook_assembly(0x0abb2b, 14, target);
        memory_accessor.virtual_protect(0x0abb2b, 14, old)?;
        Ok(())
    }

    pub fn hook_120db5(&self, target: usize) -> Result<usize> {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hook_120db5 is only available for HookedProcess");
        };
        let old = memory_accessor.virtual_protect(0x120db5, 5, PAGE_EXECUTE_WRITECOPY)?;
        let original = memory_accessor.hook_call(0x120db5, target);
        memory_accessor.virtual_protect(0x120db5, 5, old)?;
        Ok(original)
    }

    pub fn input_mut(&self) -> &'static mut Input {
        debug_assert_eq!(0x03d4, size_of::<InputDevice>());
        debug_assert_eq!(0x2e2c, size_of::<Input>());

        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::hook_120db5 is only available for HookedProcess");
        };
        let p_p_input = memory_accessor.raw_ptr(0x1ae3a0) as *const *mut Input;
        unsafe { (*p_p_input).as_mut().unwrap() }
    }

    u16_prop!(0x1ae410, rand_seed1, set_rand_seed1);
    u16_prop!(0x1ae430, rand_seed2, set_rand_seed2);
    u16_prop!(0x200850, p1_input);
    u16_prop!(0x200b10, p2_input);

    pub fn direct_3d_device(&self) -> Result<&'static IDirect3DDevice9> {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            panic!("Th19::direct_3d_device is only available for HookedProcess");
        };
        unsafe {
            IDirect3DDevice9::from_raw_borrowed(
                &*(memory_accessor.raw_ptr(0x208388) as *const *mut c_void),
            )
        }
        .ok_or_else(|| anyhow!("IDirect3DDevice9::from_raw_borrowed failed"))
    }
}
