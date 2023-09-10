mod find_process_id;
pub mod inject_dll;
pub mod memory_accessors;
mod win_api_wrappers;

use anyhow::{bail, Result};
use memory_accessors::{ExternalProcess, HookedProcess, MemoryAccessor};
use windows::Win32::{
    Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY,
};

pub struct Th19 {
    memory_accessor: MemoryAccessor,
}

macro_rules! u16_prop {
    ($addr:expr, $getter:ident, $setter:ident) => {
        pub fn $getter(&self) -> Result<u16> {
            self.memory_accessor.read_u16($addr)
        }
        pub fn $setter(&self, value: u16) -> Result<()> {
            self.memory_accessor.write_u16($addr, value)
        }
    };
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
            bail!("Th19::hook_0a96b5 is only available for HookedProcess");
        };
        let old = memory_accessor.virtual_protect(0x0a96b5, 0x05, PAGE_EXECUTE_WRITECOPY)?;
        let original = memory_accessor.hook_func(0x0a96b5, target);
        memory_accessor.virtual_protect(0x0a96b5, 0x05, old)?;
        Ok(original)
    }

    u16_prop!(0x1ae410, rand_seed1, set_rand_seed1);
    u16_prop!(0x1ae430, rand_seed2, set_rand_seed2);
    u16_prop!(0x200850, p1_input, set_p1_input);
    u16_prop!(0x200b10, p2_input, set_p2_input);

    pub fn direct_3d_device(&self) -> Result<&'static IDirect3DDevice9> {
        let MemoryAccessor::HookedProcess(memory_accessor) = &self.memory_accessor else {
            bail!("Th19::direct_3d_device is only available for HookedProcess");
        };
        memory_accessor.as_direct_3d_device(0x208388)
    }
}
