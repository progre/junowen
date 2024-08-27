use std::env::args;

use anyhow::Result;

use junowen_lib::Th19;

fn main() -> Result<()> {
    let mut args = args();
    args.next();
    let seed1 = args.next().and_then(|x| x.parse::<u32>().ok());
    let seed2 = args.next().and_then(|x| x.parse::<u32>().ok());
    let mut th19 = Th19::new_external_process("th19.exe")?;
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
