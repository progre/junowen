use std::borrow::Cow;

use crate::signaling::waiting_for_match::{WaitingForPureP2pSpectator, WaitingForSpectator};

use super::SpectatorHostState;

/// 観戦者の受け付け状況を表示するためのテキスト (下線, 本文) を返す
pub fn status_text(state: &SpectatorHostState) -> (&'static str, Cow<'static, str>) {
    let spectators = state.count_spectators();
    if spectators > 0 {
        return (
            "               ",
            Cow::Owned(format!("Spectator(s): {}", spectators)),
        );
    }
    let Some(WaitingForSpectator::PureP2p(waiting)) = state.waiting() else {
        return ("", "".into());
    };
    match waiting {
        WaitingForPureP2pSpectator::Standby { ready: false, .. }
        | WaitingForPureP2pSpectator::SignalingCodeRecved { ready: false, .. }
        | WaitingForPureP2pSpectator::SignalingCodeSent { ready: false, .. } => ("", "".into()),
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
    }
}
