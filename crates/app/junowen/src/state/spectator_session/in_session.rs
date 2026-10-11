use std::ffi::c_void;

use junowen_lib::Th19;

use crate::state::{
    render_parts::{render_footer, render_names},
    spectator_host::{SpectatorHostState, status_text},
};

pub fn on_render_texts_spectator(
    th19: &Th19,
    text_renderer: &c_void,
    p1_name: &str,
    p2_name: &str,
    waiting_for_next_game: bool,
    relay: Option<&SpectatorHostState>,
) {
    render_names(th19, text_renderer, p1_name, p2_name);
    let spectating = if waiting_for_next_game {
        "(Spectating: waiting for next game...)"
    } else {
        "(Spectating)"
    };
    let (msg2_rear, msg2_front) = relay.map(status_text).unwrap_or_default();
    let msg_front/* _ */= format!("{} {}", spectating, msg2_front);
    let msg_rear/* __ */= format!("{} {}", " ".repeat(spectating.len()), msg2_rear);
    render_footer(th19, text_renderer, &msg_front, &msg_rear);
}

pub fn on_render_texts_waiting_for_host(th19: &Th19, text_renderer: &c_void) {
    render_footer(th19, text_renderer, "(Waiting for host...)", "");
}
