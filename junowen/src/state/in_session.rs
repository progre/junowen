use std::{borrow::Cow, ffi::c_void};

use junowen_lib::{Fn10f720, RenderingText, Th19};
use tracing::trace;

use crate::in_game_lobby::waiting_for_spectator::{
    WaitingForPureP2pSpectator, WaitingForSpectator,
};

use super::spectator_host::SpectatorHostState;

fn render_names(text_renderer: *const c_void, th19: &Th19, p1_name: &str, p2_name: &str) {
    let mut text = RenderingText::default();
    text.set_text(p1_name.as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (4 * th19.screen_height().unwrap() / 1280) as f32;
    text.color = 0xffff8080;
    th19.render_text(text_renderer, &text);

    text.set_text(p2_name.as_bytes());
    text.x = (1264 * th19.screen_width().unwrap() / 1280) as f32;
    text.color = 0xff8080ff;
    text.horizontal_align = 2;
    th19.render_text(text_renderer, &text);
}

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
                "            ",
                Cow::Owned(format!(
                    "Spectator: {}",
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

pub fn on_render_texts_spectator(
    th19: &Th19,
    p1_name: &str,
    p2_name: &str,
    text_renderer: *const c_void,
) {
    let msg2 = "(Spectating)";
    let mut text = RenderingText::default();
    text.set_text(format!("Ju.N.Owen v{} {}", env!("CARGO_PKG_VERSION"), msg2).as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (940 * th19.screen_height().unwrap() / 960) as f32;
    text.color = 0xffffffff;
    text.font_type = 1;
    th19.render_text(text_renderer, &text);

    render_names(text_renderer, th19, p1_name, p2_name);
}

pub fn on_rewrite_controller_assignments(th19: &mut Th19, old_fn: fn(&mut Th19) -> Fn10f720) {
    let input_devices = th19.input_devices_mut();
    let old_p1_idx = input_devices.p1_idx();
    trace!(
        "on_rewrite_controller_assignments: before old_p1_idx={}",
        old_p1_idx
    );
    old_fn(th19)();
    if old_p1_idx == 0 && input_devices.p1_idx() != 0 {
        trace!(
            "on_rewrite_controller_assignments: after input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
        input_devices.set_p1_idx(0);
        trace!(
            "on_rewrite_controller_assignments: fixed input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
    }
}
