//! ロビー画面
//!
//! 描画・効果音・キーボード入力はゲームの機能を使うため、トレイトを介して利用側から受け取る

mod helper;
mod menu;
mod overlay_text;
pub mod pure_p2p_offerer_menu;
mod scene;
pub mod tcp_signaling_menu;
mod text_layout;

pub use helper::{overlay_description, render_text_line, signaling_code_line_count};
pub use menu::{
    MenuButtons, MenuDevice, MenuRenderer, MenuSound, MenuText, menu_item_color, render_label_value,
};
pub use overlay_text::OverlayText;
pub use scene::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult};
pub use text_layout::TextLayout;
