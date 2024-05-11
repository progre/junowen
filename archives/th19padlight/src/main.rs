use std::env::current_exe;

use anyhow::Result;
use junowen_lib::hook_utils::do_dll_injection;

fn main() -> Result<()> {
    let dll_path = current_exe()?
        .as_path()
        .parent()
        .unwrap()
        .join(concat!(env!("CARGO_PKG_NAME"), "_hook.dll"));

    do_dll_injection("th19.exe", &dll_path)?;

    Ok(())
}
