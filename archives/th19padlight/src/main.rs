use std::env::current_exe;

use anyhow::Result;
use junowen_lib::hook_utils::inject_dll;

fn main() -> Result<()> {
    let dll_path = current_exe()?
        .as_path()
        .parent()
        .unwrap()
        .join(concat!(env!("CARGO_PKG_NAME"), "_hook.dll"));

    inject_dll("th19.exe", &dll_path)?;

    Ok(())
}
