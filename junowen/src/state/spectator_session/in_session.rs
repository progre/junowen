use std::ffi::c_void;

use junowen_lib::Th19;

use crate::state::render_parts::{render_footer, render_names};

pub fn on_render_texts_spectator(
    th19: &Th19,
    text_renderer: &c_void,
    p1_name: &str,
    p2_name: &str,
    waiting_for_next_game: bool,
) {
    render_names(th19, text_renderer, p1_name, p2_name);
    let msg = if waiting_for_next_game {
        "(Spectating: waiting for next game...)"
    } else {
        "(Spectating)"
    };
    render_footer(th19, text_renderer, msg, "");
}

pub fn on_render_texts_waiting_for_host(th19: &Th19, text_renderer: &c_void) {
    render_footer(th19, text_renderer, "(Waiting for host...)", "");
}
