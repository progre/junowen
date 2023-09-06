mod external_process;

use anyhow::Result;

pub use external_process::ExternalProcess;

pub trait MemoryAccessor {
    fn read(&self, address: usize, buffer: &mut [u8]) -> Result<()>;
    fn write(&self, address: usize, buffer: &[u8]) -> Result<()>;
    fn read_u16(&self, address: usize) -> Result<u16> {
        let mut buffer = [0; 2];
        self.read(address, &mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }
    fn write_u16(&self, address: usize, value: u16) -> Result<()> {
        self.write(address, &value.to_le_bytes())
    }
}
