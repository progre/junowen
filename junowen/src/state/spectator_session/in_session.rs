use std::ffi::c_void;

use junowen_lib::Th19;

use crate::state::{
    battle_session::{in_session::spectator_host_status, spectator_host::SpectatorHostState},
    render_parts::{render_footer, render_names},
};

pub fn on_render_texts_spectator(
    th19: &Th19,
    text_renderer: &c_void,
    p1_name: &str,
    p2_name: &str,
    relay: Option<&SpectatorHostState>,
) {
    render_names(th19, text_renderer, p1_name, p2_name);
    let (msg2_rear, msg2_front) = relay.map(spectator_host_status).unwrap_or_default();
    let msg_front/* _ */= format!("(Spectating) {}", msg2_front);
    let msg_rear/* __ */= format!("             {}", msg2_rear);
    render_footer(th19, text_renderer, &msg_front, &msg_rear);
}

pub fn on_render_texts_waiting_for_host(th19: &Th19, text_renderer: &c_void) {
    render_footer(th19, text_renderer, "(Waiting for host...)", "");
}
