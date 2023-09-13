use std::{ffi::c_void, mem::transmute};

use crate::{
    file::{read_from_file, write_to_file},
    prop, state, state_mut,
};

// 1. 画面を開くときに本来の値をメモリーから退避し、ファイルの値をメモリーに適用する
// 2. 画面を閉じるときにメモリーの値をファイルに書き出し、本来の値をメモリーに戻す
// 既知の不具合: 編集中に正規の手段で終了すると値が保存されてしまう

pub extern "thiscall" fn on_open_settings_editor(this: *const c_void, arg1: u32) -> u32 {
    type Func = extern "thiscall" fn(this: *const c_void, arg1: u32) -> u32;
    let prop = prop();
    let func: Func = unsafe { transmute(prop.original_fn_from_107540_0046) };
    if prop.th19.is_network_mode() {
        return func(this, arg1);
    }

    // ファイルから読み込んだ設定を適用
    state_mut().tmp_battle_settings = prop.th19.battle_settings_in_menu().unwrap();
    let settings_of_file = read_from_file(&prop.settings_path)
        .or_else(|_| prop.th19.battle_settings_in_menu())
        .unwrap();
    prop.th19
        .set_battle_settings_in_menu(&settings_of_file)
        .unwrap();

    func(this, arg1)
}

pub extern "thiscall" fn on_close_settings_editor(this: *const c_void) {
    type Func = extern "thiscall" fn(this: *const c_void);
    let prop = prop();
    let func: Func = unsafe { transmute(prop.original_fn_from_107540_0937) };
    if prop.th19.is_network_mode() {
        return func(this);
    }

    // ファイルに書き出し
    let current = prop.th19.battle_settings_in_menu().unwrap();
    write_to_file(&prop.settings_path, &current).unwrap();
    prop.th19
        .set_battle_settings_in_menu(&state().tmp_battle_settings)
        .unwrap();

    func(this)
}
