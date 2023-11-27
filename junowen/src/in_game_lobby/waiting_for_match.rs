mod reserved_room_opponent_socket;
pub mod rooms;
mod shared_room_opponent_socket;
mod socket;

use derive_new::new;
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::session::{battle::BattleSession, spectator::SpectatorSessionGuest};

use self::rooms::{WaitingForOpponentInReservedRoom, WaitingForOpponentInSharedRoom};

#[derive(new)]
pub struct WaitingForPureP2pOpponent {
    battle_session_rx: mpsc::Receiver<BattleSession>,
}

pub enum WaitingForOpponent {
    SharedRoom(WaitingForOpponentInSharedRoom),
    ReservedRoom(WaitingForOpponentInReservedRoom),
    PureP2p(WaitingForPureP2pOpponent),
}

impl WaitingForOpponent {
    pub fn try_into_session(self) -> Result<BattleSession, Self> {
        match self {
            Self::SharedRoom(room) => room
                .try_into_session()
                .map_err(WaitingForOpponent::SharedRoom),
            Self::ReservedRoom(room) => room
                .try_into_session()
                .map_err(WaitingForOpponent::ReservedRoom),
            Self::PureP2p(mut pure_p2p) => pure_p2p
                .battle_session_rx
                .try_recv()
                .map_err(|_| WaitingForOpponent::PureP2p(pure_p2p)),
        }
    }
}

#[derive(new)]
pub struct WaitingForPureP2pSpectator {
    spectator_session_guest_rx: mpsc::Receiver<SpectatorSessionGuest>,
}

pub enum WaitingForSpectator {
    PureP2p(WaitingForPureP2pSpectator),
}

impl WaitingForSpectator {
    pub fn try_recv_session(&mut self) -> Result<SpectatorSessionGuest, TryRecvError> {
        match self {
            Self::PureP2p(pure_p2p) => pure_p2p.spectator_session_guest_rx.try_recv(),
        }
    }
}

pub enum WaitingForMatch {
    Opponent(WaitingForOpponent),
    Spectator(WaitingForSpectator),
}

impl From<WaitingForPureP2pOpponent> for WaitingForMatch {
    fn from(value: WaitingForPureP2pOpponent) -> Self {
        WaitingForMatch::Opponent(WaitingForOpponent::PureP2p(value))
    }
}

impl From<WaitingForPureP2pSpectator> for WaitingForMatch {
    fn from(value: WaitingForPureP2pSpectator) -> Self {
        WaitingForMatch::Spectator(WaitingForSpectator::PureP2p(value))
    }
}
