use junowen_lib::{Th19, structs::app::MainMenu};
use tokio::sync::mpsc::error::TryRecvError;

use crate::session::{spectator::SpectatorRelayRoom, spectator_host::SpectatorHostSession};

use super::{
    waiting_for_pure_p2p_spectator::WaitingForPureP2pSpectator,
    waiting_in_room::WaitingForSpectatorInReservedRoom,
};

pub enum WaitingForSpectator {
    PureP2p(WaitingForPureP2pSpectator),
    ReservedRoom {
        room: SpectatorRelayRoom,
        waiting: WaitingForSpectatorInReservedRoom,
    },
}

impl WaitingForSpectator {
    /// 専有ルームの情報があれば専有ルームで、無ければ Pure P2P で観戦者を待ち受ける
    pub fn new(room: Option<SpectatorRelayRoom>) -> Self {
        match room {
            Some(room) => {
                let waiting = WaitingForSpectatorInReservedRoom::new(
                    room.room_name().clone(),
                    room.key().clone(),
                );
                Self::ReservedRoom { room, waiting }
            }
            None => Self::PureP2p(WaitingForPureP2pSpectator::standby()),
        }
    }

    pub fn room(&self) -> Option<&SpectatorRelayRoom> {
        match self {
            Self::PureP2p(_) => None,
            Self::ReservedRoom { room, .. } => Some(room),
        }
    }

    /// 専有ルームの場合、観戦者を受信した後は再度 `new` で待ち受けを作り直す必要がある
    pub fn try_recv_session(
        &mut self,
        pushed: bool,
        menu: Option<&MainMenu>,
        th19: &Th19,
    ) -> Option<SpectatorHostSession> {
        match self {
            Self::PureP2p(waiting) => {
                waiting.update(pushed, menu, th19);
                match waiting {
                    WaitingForPureP2pSpectator::Standby { .. }
                    | WaitingForPureP2pSpectator::SignalingCodeRecved { .. } => None,
                    WaitingForPureP2pSpectator::SignalingCodeSent { session_rx, .. } => {
                        match session_rx.try_recv() {
                            Err(TryRecvError::Empty) => None,
                            Err(TryRecvError::Disconnected) => {
                                *self = Self::PureP2p(WaitingForPureP2pSpectator::standby());
                                None
                            }
                            Ok(session) => {
                                *self = Self::PureP2p(WaitingForPureP2pSpectator::standby());
                                Some(session)
                            }
                        }
                    }
                }
            }
            Self::ReservedRoom { waiting, .. } => waiting.try_recv_session(),
        }
    }
}
