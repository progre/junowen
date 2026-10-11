//! ロビー画面のメニュー部品
//!
//! 描画・効果音・キーボード入力はゲームの機能を使うため、トレイトを介して利用側から受け取る

mod action;
mod common_menu;
mod device;
mod input;
mod menu;
mod render;

pub use action::{Action, OnMenuInputResult};
pub use common_menu::CommonMenu;
pub use device::{MenuDevice, MenuSound};
pub use input::{MenuButtons, MenuController, TextInput};
pub use menu::{
    Menu, MenuItem, MenuPlainItem, MenuSubMenuItem, MenuSubSceneItem, MenuTextInputItem,
};
pub use render::{MenuRenderer, MenuText, menu_item_color, render_label_value};
