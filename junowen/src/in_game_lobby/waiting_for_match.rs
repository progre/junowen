mod reserved_room_opponent_socket;
mod reserved_room_spectator_socket;
pub mod rooms;
mod shared_room_opponent_socket;
mod socket;

use derive_new::new;
use tokio::sync::mpsc;

use crate::session::{battle::BattleSession, spectator::SpectatorSessionGuest};

use self::rooms::{
    WaitingForOpponentInReservedRoom, WaitingForOpponentInSharedRoom,
    WaitingForSpectatorHostInReservedRoom,
};

use super::waiting_for_spectator::{WaitingForPureP2pSpectator, WaitingForSpectator};

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
    pub fn try_into_session_and_waiting_for_spectator(
        self,
    ) -> Result<(BattleSession, WaitingForSpectator), Self> {
        match self {
            Self::SharedRoom(waiting) => waiting
                .try_into_session()
                .map(|session| {
                    (
                        session,
                        WaitingForSpectator::PureP2p(WaitingForPureP2pSpectator::standby()),
                    )
                })
                .map_err(WaitingForOpponent::SharedRoom),
            Self::ReservedRoom(waiting) => waiting
                .try_into_session_and_waiting_for_spectator()
                .map_err(WaitingForOpponent::ReservedRoom),
            Self::PureP2p(mut waiting) => waiting
                .battle_session_rx
                .try_recv()
                .map(|session| {
                    (
                        session,
                        WaitingForSpectator::PureP2p(WaitingForPureP2pSpectator::standby()),
                    )
                })
                .map_err(|_| Self::PureP2p(waiting)),
        }
    }
}

#[derive(new)]
pub struct WaitingForPureP2pSpectatorHost {
    spectator_session_guest_rx: mpsc::Receiver<SpectatorSessionGuest>,
}

pub enum WaitingForSpectatorHost {
    PureP2p(WaitingForPureP2pSpectatorHost),
    ReservedRoom(WaitingForSpectatorHostInReservedRoom),
}

impl WaitingForSpectatorHost {
    pub fn try_into_session(self) -> Result<SpectatorSessionGuest, Self> {
        match self {
            Self::PureP2p(mut waiting) => waiting
                .spectator_session_guest_rx
                .try_recv()
                .map_err(|_| WaitingForSpectatorHost::PureP2p(waiting)),
            Self::ReservedRoom(waiting) => waiting
                .try_into_session()
                .map_err(WaitingForSpectatorHost::ReservedRoom),
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