use std::env::args;

use anyhow::Result;

use junowen::memory_accessors::{ExternalProcess, MemoryAccessor};

struct Th19<T>
where
    T: MemoryAccessor,
{
    memory_accessor: T,
}

impl<T> Th19<T>
where
    T: MemoryAccessor,
{
    pub fn new(memory_accessor: T) -> Self {
        Self { memory_accessor }
    }

    pub fn rand_seed1(&self) -> Result<u16> {
        self.memory_accessor.read_u16(0x1ae410)
    }
    pub fn set_rand_seed1(&self, value: u16) -> Result<()> {
        self.memory_accessor.write_u16(0x1ae410, value)
    }

    pub fn rand_seed2(&self) -> Result<u16> {
        self.memory_accessor.read_u16(0x1ae430)
    }
    pub fn set_rand_seed2(&self, value: u16) -> Result<()> {
        self.memory_accessor.write_u16(0x1ae430, value)
    }
}

fn main() -> Result<()> {
    let mut args = args();
    args.next();
    let seed1 = args.next().and_then(|x| x.parse::<u16>().ok());
    let seed2 = args.next().and_then(|x| x.parse::<u16>().ok());
    let th19 = Th19::new(ExternalProcess::new()?);
    if let (Some(seed1), Some(seed2)) = (seed1, seed2) {
        th19.set_rand_seed1(seed1)?;
        th19.set_rand_seed2(seed2)?;
        Ok(())
    } else {
        let seed1 = th19.rand_seed1()?;
        let seed2 = th19.rand_seed2()?;
        println!("seed: {} {}", seed1, seed2);
        Ok(())
    }
}
