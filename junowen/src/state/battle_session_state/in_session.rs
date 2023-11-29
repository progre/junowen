use std::{borrow::Cow, ffi::c_void};

use junowen_lib::{RenderingText, Th19};

use crate::{
    in_game_lobby::{WaitingForPureP2pSpectator, WaitingForSpectator},
    state::render_names::render_names,
};

use super::spectator_host::SpectatorHostState;

pub fn on_render_texts(
    th19: &Th19,
    host: bool,
    delay: u8,
    p1_name: &str,
    p2_name: &str,
    spectator_host_state: Option<&SpectatorHostState>,
    text_renderer: *const c_void,
) {
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

    let version = env!("CARGO_PKG_VERSION");
    let version_blank = (0..version.len()).map(|_| " ").collect::<String>();
    let delay_underline = if host { "_" } else { " " };
    let msg_rear = format!(
        "           {}        {} {}",
        version_blank, delay_underline, msg2_rear
    );
    let msg_front = format!("Ju.N.Owen v{} Delay: {} {}", version, delay, msg2_front);

    let mut text = RenderingText::default();
    text.set_text(msg_rear.as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (944 * th19.screen_height().unwrap() / 960) as f32;
    text.color = 0xffffffff;
    text.font_type = 1;
    th19.render_text(text_renderer, &text);

    text.set_text(msg_front.as_bytes());
    text.y = (940 * th19.screen_height().unwrap() / 960) as f32;
    th19.render_text(text_renderer, &text);

    render_names(text_renderer, th19, p1_name, p2_name);
}
