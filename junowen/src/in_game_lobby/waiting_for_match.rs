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
            Self::SharedRoom(waiting) => waiting
                .try_into_session()
                .map_err(WaitingForOpponent::SharedRoom),
            Self::ReservedRoom(waiting) => waiting
                .try_into_session()
                .map_err(WaitingForOpponent::ReservedRoom),
            Self::PureP2p(mut waiting) => waiting
                .battle_session_rx
                .try_recv()
                .map_err(|_| WaitingForOpponent::PureP2p(waiting)),
        }
    }
}

#[derive(new)]
pub struct WaitingForPureP2pSpectatorHost {
    spectator_session_guest_rx: mpsc::Receiver<SpectatorSessionGuest>,
}

pub enum WaitingForSpectatorHost {
    PureP2p(WaitingForPureP2pSpectatorHost),
}

impl WaitingForSpectatorHost {
    pub fn try_recv_session(&mut self) -> Result<SpectatorSessionGuest, TryRecvError> {
        match self {
            Self::PureP2p(waiting) => waiting.spectator_session_guest_rx.try_recv(),
        }
    }
}

pub enum WaitingForMatch {
    Opponent(WaitingForOpponent),
    SpectatorHost(WaitingForSpectatorHost),
}

impl From<WaitingForPureP2pOpponent> for WaitingForMatch {
    fn from(value: WaitingForPureP2pOpponent) -> Self {
        WaitingForMatch::Opponent(WaitingForOpponent::PureP2p(value))
    }
}

impl From<WaitingForPureP2pSpectatorHost> for WaitingForMatch {
    fn from(value: WaitingForPureP2pSpectatorHost) -> Self {
        WaitingForMatch::SpectatorHost(WaitingForSpectatorHost::PureP2p(value))
    }
}
