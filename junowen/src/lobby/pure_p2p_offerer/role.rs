use junowen_lib::connection::signaling::SignalingCodeType;

use crate::session::{battle::BattleSession, spectator::SpectatorSession};

use super::PureP2pOfferer;

/// 相手の役割によって変わるメッセージの翻訳キー
#[derive(Clone, Copy)]
pub struct Messages {
    pub share: &'static str,
    pub opponent_code: &'static str,
    pub waiting: &'static str,
}

pub fn pure_p2p_host() -> PureP2pOfferer<BattleSession> {
    PureP2pOfferer::new(
        SignalingCodeType::BattleOffer,
        SignalingCodeType::BattleAnswer,
        |pc, dc| BattleSession::new(pc, dc, true),
        "Connect as a Host",
        Messages {
            share: "pure_p2p.host_share",
            opponent_code: "pure_p2p.host_opponent_code",
            waiting: "pure_p2p.host_waiting",
        },
    )
}

pub fn pure_p2p_spectator() -> PureP2pOfferer<SpectatorSession> {
    PureP2pOfferer::new(
        SignalingCodeType::SpectatorOffer,
        SignalingCodeType::SpectatorAnswer,
        SpectatorSession::new,
        "Connect as a Spectator",
        Messages {
            share: "pure_p2p.spectator_share",
            opponent_code: "pure_p2p.spectator_opponent_code",
            waiting: "pure_p2p.spectator_waiting",
        },
    )
}
