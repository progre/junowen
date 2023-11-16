use derive_new::new;
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::session::{battle::BattleSession, spectator::SpectatorSessionGuest};

#[derive(new)]
pub struct PureP2pOpponent {
    battle_session_rx: mpsc::Receiver<BattleSession>,
}

pub enum Opponent {
    PureP2p(PureP2pOpponent),
}

impl Opponent {
    pub fn try_into_session(self) -> Result<BattleSession, Self> {
        match self {
            Self::PureP2p(mut pure_p2p) => pure_p2p
                .battle_session_rx
                .try_recv()
                .map_err(|_| Opponent::PureP2p(pure_p2p)),
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
    pub fn try_recv_session(&mut self) -> Result<SpectatorSessionGuest, TryRecvError> {
        match self {
            Self::PureP2p(pure_p2p) => pure_p2p.spectator_session_guest_rx.try_recv(),
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
