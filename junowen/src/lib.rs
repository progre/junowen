mod find_process_id;
pub mod inject_dll;
pub mod memory_accessors;
mod win_api_wrappers;

use anyhow::Result;
use memory_accessors::MemoryAccessor;
use windows::Win32::{
    Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_EXECUTE_WRITECOPY,
};

pub struct Th19<T>
where
    T: MemoryAccessor,
{
    memory_accessor: T,
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

impl<T> Th19<T>
where
    T: MemoryAccessor,
{
    pub fn new(memory_accessor: T) -> Self {
        Self { memory_accessor }
    }

    /// # Safety
    pub unsafe fn hook_0a96b5(&self, target: usize) -> Result<usize> {
        let old = self
            .memory_accessor
            .virtual_protect(0x0a96b5, 0x05, PAGE_EXECUTE_WRITECOPY)?;
        let original = self.memory_accessor.hook_func(0x0a96b5, target);
        self.memory_accessor.virtual_protect(0x0a96b5, 0x05, old)?;
        Ok(original)
    }

    u16_prop!(0x1ae410, rand_seed1, set_rand_seed1);
    u16_prop!(0x1ae430, rand_seed2, set_rand_seed2);
    u16_prop!(0x200850, p1_input, set_p1_input);
    u16_prop!(0x200b10, p2_input, set_p2_input);

    pub fn direct_3d_device(&self) -> Result<&'static IDirect3DDevice9> {
        self.memory_accessor.as_direct_3d_device(0x208388)
    }
}
