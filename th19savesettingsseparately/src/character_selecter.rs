use std::ffi::c_void;

use crate::{file::read_from_file, props, state_mut};

pub extern "thiscall" fn post_read_battle_settings_from_menu_to_game(
    this: *const c_void,
    arg1: u32,
) -> u32 {
    let prop = props();
    let th19 = &mut state_mut().th19;
    let func = prop.original_fn_from_13f9d0_0446;
    if th19.is_network_mode() {
        return func(this, arg1);
    }

    // ファイルから読み込んだ設定を適用
    let battle_settings = read_from_file(&prop.settings_path)
        .or_else(|_| th19.game_settings_in_menu())
        .unwrap();
    th19.put_game_settings_in_game(&battle_settings).unwrap();

    func(this, arg1)
}
