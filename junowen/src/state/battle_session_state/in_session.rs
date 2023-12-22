use std::{borrow::Cow, ffi::c_void};

use junowen_lib::Th19;

use crate::{
    signaling::waiting_for_match::{WaitingForPureP2pSpectator, WaitingForSpectator},
    state::render_parts::{render_footer, render_names},
};

use super::spectator_host::SpectatorHostState;

pub fn on_render_texts(
    th19: &Th19,
    text_renderer: *const c_void,
    host: bool,
    delay: u8,
    p1_name: &str,
    p2_name: &str,
    spectator_host_state: Option<&SpectatorHostState>,
) {
    render_names(th19, text_renderer, p1_name, p2_name);

    let (msg2_rear, msg2_front) = if let Some(spectator_host_state) = spectator_host_state {
        if spectator_host_state.count_spectators() > 0 {
            (
                "               ",
                Cow::Owned(format!(
                    "Spectator(s): {}",
                    spectator_host_state.count_spectators()
                )),
            )
        } else {
            match spectator_host_state.waiting() {
                WaitingForSpectator::PureP2p(waiting) => match waiting {
                    WaitingForPureP2pSpectator::Standby { ready: false, .. }
                    | WaitingForPureP2pSpectator::SignalingCodeRecved { ready: false, .. }
                    | WaitingForPureP2pSpectator::SignalingCodeSent { ready: false, .. } => {
                        ("", "".into())
                    }
                    WaitingForPureP2pSpectator::Standby { .. } => (
                        "       __                                    ",
                        "(Press F1 to accept spectator from clipboard)".into(),
                    ),
                    WaitingForPureP2pSpectator::SignalingCodeRecved { .. } => (
                        "                              ",
                        "(Generating signaling code...)".into(),
                    ),
                    WaitingForPureP2pSpectator::SignalingCodeSent { .. } => (
                        "                                                      ",
                        "(Your signaling code has been copied to the clipboard)".into(),
                    ),
                },
                WaitingForSpectator::ReservedRoom(_) => ("", "".into()),
            }
        }
    } else {
        ("", "".into())
    };

    let delay_underline = if host { "_" } else { " " };
    let msg_front/* _ */= format!("Delay: {} {}", delay, msg2_front);
    let msg_rear/* __ */= format!("       {} {}", delay_underline, msg2_rear);

    render_footer(th19, text_renderer, &msg_front, &msg_rear);
}
