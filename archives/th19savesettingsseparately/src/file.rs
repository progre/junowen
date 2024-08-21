use std::mem::transmute;

use anyhow::{bail, Result};
use junowen_lib::structs::settings::GameSettings;

pub fn read_from_file(settings_path: &str) -> Result<GameSettings> {
    let vec = std::fs::read(settings_path)?;
    if vec.len() != 12 {
        bail!("Invalid file size");
    }
    let mut bytes = [0u8; 12];
    bytes.copy_from_slice(&vec);
    Ok(unsafe { transmute::<[u8; 12], GameSettings>(bytes) })
}

pub fn write_to_file(settings_path: &str, battle_settings: &GameSettings) -> Result<()> {
    let contents: &[u8; 12] = unsafe { transmute(battle_settings) };
    Ok(std::fs::write(settings_path, contents)?)
}
