mod external_process;
mod hooked_process;

use anyhow::Result;

pub use external_process::ExternalProcess;
pub use hooked_process::HookedProcess;
use windows::Win32::{
    Graphics::Direct3D9::IDirect3DDevice9, System::Memory::PAGE_PROTECTION_FLAGS,
};

pub trait MemoryAccessor {
    fn read(&self, addr: usize, buffer: &mut [u8]) -> Result<()>;
    fn write(&self, addr: usize, buffer: &[u8]) -> Result<()>;
    fn as_direct_3d_device(&self, addr: usize) -> Result<&'static IDirect3DDevice9>;
    /// # Safety
    unsafe fn virtual_protect(
        &self,
        addr: usize,
        size: usize,
        protect: PAGE_PROTECTION_FLAGS,
    ) -> Result<PAGE_PROTECTION_FLAGS>;
    /// # Safety
    unsafe fn hook_func(&self, addr: usize, target: usize) -> usize;

    fn read_u16(&self, addr: usize) -> Result<u16> {
        let mut buffer = [0; 2];
        self.read(addr, &mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }

    fn write_u16(&self, addr: usize, value: u16) -> Result<()> {
        self.write(addr, &value.to_le_bytes())
    }
}
