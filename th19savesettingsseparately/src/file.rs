use std::mem::transmute;

use anyhow::{bail, Result};
use junowen_lib::BattleSettings;

pub fn read_from_file(settings_path: &str) -> Result<BattleSettings> {
    let vec = std::fs::read(settings_path)?;
    if vec.len() != 12 {
        bail!("Invalid file size");
    }
    let mut bytes = [0u8; 12];
    bytes.copy_from_slice(&vec);
    Ok(unsafe { transmute(bytes) })
}

pub fn write_to_file(settings_path: &str, battle_settings: &BattleSettings) -> Result<()> {
    let contents: &[u8; 12] = unsafe { transmute(battle_settings) };
    Ok(std::fs::write(settings_path, contents)?)
}
