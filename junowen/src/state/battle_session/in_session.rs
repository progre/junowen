use std::{borrow::Cow, ffi::c_void};

use junowen_lib::{Th19, structs::settings::GameSettings};

use crate::{
    signaling::waiting_for_match::{WaitingForPureP2pSpectator, WaitingForSpectator},
    state::render_parts::{render_footer, render_game_settings, render_names},
};

use super::spectator_host::SpectatorHostState;

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

    let spectators = status.spectator_host_state.count_spectators();
    let pending_spectators = status.spectator_host_state.count_pending_spectators();
    let (msg2_rear, msg2_front): (Cow<str>, Cow<str>) = if spectators > 0 || pending_spectators > 0
    {
        let msg = if pending_spectators > 0 {
            format!(
                "Spectator(s): {} (+{} joining)",
                spectators, pending_spectators
            )
        } else {
            format!("Spectator(s): {}", spectators)
        };
        (" ".repeat(msg.len()).into(), msg.into())
    } else {
        match status.spectator_host_state.waiting() {
            WaitingForSpectator::PureP2p(waiting) => match waiting {
                WaitingForPureP2pSpectator::Standby { ready: false, .. }
                | WaitingForPureP2pSpectator::SignalingCodeRecved { ready: false, .. }
                | WaitingForPureP2pSpectator::SignalingCodeSent { ready: false, .. } => {
                    ("".into(), "".into())
                }
                WaitingForPureP2pSpectator::Standby { .. } => (
                    "       __                                    ".into(),
                    "(Press F1 to accept spectator from clipboard)".into(),
                ),
                WaitingForPureP2pSpectator::SignalingCodeRecved { .. } => (
                    "                              ".into(),
                    "(Generating signaling code...)".into(),
                ),
                WaitingForPureP2pSpectator::SignalingCodeSent { .. } => (
                    "                                                      ".into(),
                    "(Your signaling code has been copied to the clipboard)".into(),
                ),
            },
            WaitingForSpectator::ReservedRoom(_) => ("".into(), "".into()),
        }
    };

    let delay_underline = if status.host { "_" } else { " " };
    let msg_front/* _ */= format!("Delay: {} {}", status.delay, msg2_front);
    let msg_rear/* __ */= format!("       {} {}", delay_underline, msg2_rear);

    render_footer(th19, text_renderer, &msg_front, &msg_rear);
}
