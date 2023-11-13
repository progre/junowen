use derive_new::new;
use tokio::sync::mpsc;

use crate::session::{battle::BattleSession, spectator::SpectatorSessionGuest};

#[derive(new)]
pub struct PureP2pOpponent {
    battle_session_rx: mpsc::Receiver<BattleSession>,
}

pub enum Opponent {
    PureP2p(PureP2pOpponent),
}

impl Opponent {
    pub fn battle_session_rx_mut(&mut self) -> &mut mpsc::Receiver<BattleSession> {
        match self {
            Self::PureP2p(pure_p2p) => &mut pure_p2p.battle_session_rx,
        }
    }
}

#[derive(new)]
pub struct PureP2pSpectator {
    spectator_session_guest_rx: mpsc::Receiver<SpectatorSessionGuest>,
}

pub enum Spectator {
    PureP2p(PureP2pSpectator),
}

impl Spectator {
    pub fn spectator_session_guest_rx_mut(&mut self) -> &mut mpsc::Receiver<SpectatorSessionGuest> {
        match self {
            Self::PureP2p(pure_p2p) => &mut pure_p2p.spectator_session_guest_rx,
        }
    }
}

pub enum MatchStandby {
    Opponent(Opponent),
    Spectator(Spectator),
}

impl From<PureP2pOpponent> for MatchStandby {
    fn from(value: PureP2pOpponent) -> Self {
        MatchStandby::Opponent(Opponent::PureP2p(value))
    }
}

impl From<PureP2pSpectator> for MatchStandby {
    fn from(value: PureP2pSpectator) -> Self {
        MatchStandby::Spectator(Spectator::PureP2p(value))
    }
}
