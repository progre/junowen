use std::ffi::c_void;

use junowen_lib::{Th19, structs::settings::GameSettings};

use crate::state::{
    render_parts::{render_footer, render_game_settings, render_names},
    spectator_host::{SpectatorHostState, status_text},
};

pub struct RenderingStatus<'a> {
    pub host: bool,
    pub delay: u8,
    pub p1_name: &'a str,
    pub p2_name: &'a str,
    pub game_settings: Option<&'a GameSettings>,
    pub spectator_host_state: &'a SpectatorHostState,
}

pub fn on_render_texts(th19: &Th19, text_renderer: &c_void, status: RenderingStatus) {
    render_names(th19, text_renderer, status.p1_name, status.p2_name);
    if let Some(game_settings) = status.game_settings {
        render_game_settings(th19, text_renderer, game_settings);
    }

    let (msg2_rear, msg2_front) = status_text(status.spectator_host_state);

    let delay_underline = if status.host { "_" } else { " " };
    let msg_front/* _ */= format!("Delay: {} {}", status.delay, msg2_front);
    let msg_rear/* __ */= format!("       {} {}", delay_underline, msg2_rear);

    render_footer(th19, text_renderer, &msg_front, &msg_rear);
}
