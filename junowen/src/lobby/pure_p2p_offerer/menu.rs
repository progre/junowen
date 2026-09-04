use crate::lobby::common_menu::{CommonMenu, Menu, MenuItem};

const BASE_HEIGHT: u32 = 720;

pub const REGENERATE_ACTION_ID: u8 = 0;
pub const COPY_ACTION_ID: u8 = 1;
pub const PASTE_ACTION_ID: u8 = 2;

pub fn make_menu(label: &'static str) -> CommonMenu {
    let items = vec![
        MenuItem::plain("Regenerate", REGENERATE_ACTION_ID, true),
        MenuItem::plain("Copy your code", COPY_ACTION_ID, true),
        MenuItem::plain("Paste guest's code", PASTE_ACTION_ID, false),
    ];
    CommonMenu::new(false, BASE_HEIGHT, Menu::new(label, None, items, 2))
}

/// 相手のコードを受け取った後は操作するものがないため、項目を消す
pub fn make_empty_menu(label: &'static str) -> CommonMenu {
    CommonMenu::new(false, BASE_HEIGHT, Menu::new(label, None, vec![], 0))
}
