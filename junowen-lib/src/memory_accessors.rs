mod external_process;
mod hooked_process;

use anyhow::Result;

pub use external_process::ExternalProcess;
pub use hooked_process::FnOfHookAssembly;
pub use hooked_process::HookedProcess;

pub enum MemoryAccessor {
    ExternalProcess(ExternalProcess),
    HookedProcess(HookedProcess),
}

impl MemoryAccessor {
    pub fn read_u32(&self, addr: usize) -> Result<u32> {
        let mut buffer = [0; 4];
        self.read(addr, &mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    #[allow(unused)]
    pub fn write_u32(&mut self, addr: usize, value: u32) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }

    pub fn read(&self, addr: usize, buffer: &mut [u8]) -> Result<()> {
        match self {
            MemoryAccessor::ExternalProcess(accessor) => accessor.read(addr, buffer),
            MemoryAccessor::HookedProcess(accessor) => {
                accessor.read(addr, buffer);
                Ok(())
            }
        }
    }

    pub fn write(&mut self, addr: usize, buffer: &[u8]) -> Result<()> {
        match self {
            MemoryAccessor::ExternalProcess(accessor) => accessor.write(addr, buffer),
            MemoryAccessor::HookedProcess(accessor) => {
                accessor.write(addr, buffer);
                Ok(())
            }
        }
    }
}
