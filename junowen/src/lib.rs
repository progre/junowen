mod find_process_id;
pub mod inject_dll;
pub mod memory_accessors;
mod win_api_wrappers;

use anyhow::Result;
use memory_accessors::MemoryAccessor;

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

    u16_prop!(0x1ae410, rand_seed1, set_rand_seed1);
    u16_prop!(0x1ae430, rand_seed2, set_rand_seed2);
}
