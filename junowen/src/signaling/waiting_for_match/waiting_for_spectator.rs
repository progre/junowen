use std::mem;

use anyhow::Result;
use clipboard_win::{get_clipboard_string, set_clipboard_string};
use junowen_lib::{
    Th19,
    connection::signaling::{
        SignalingCodeType, parse_signaling_code,
        socket::async_read_write_socket::SignalingServerMessage,
    },
    structs::app::MainMenu,
};
use tokio::sync::mpsc::{self, error::TryRecvError};
use tracing::info;

use crate::session::{spectator::SpectatorRelayRoom, spectator_host::SpectatorHostSession};

use super::{super::Signaling, waiting_in_room::WaitingForSpectatorInReservedRoom};

fn try_start_signaling(th19: &Th19) -> Option<WaitingForPureP2pSpectator> {
    let Ok(ok) = get_clipboard_string() else {
        th19.play_sound(th19.sound_manager(), 0x10, 0);
        return None;
    };
    let Ok((SignalingCodeType::SpectatorOffer, offer)) = parse_signaling_code(&ok) else {
        th19.play_sound(th19.sound_manager(), 0x10, 0);
        return None;
    };
    let (session_tx, session_rx) = mpsc::channel(1);
    let mut signaling = Signaling::new(session_tx, SpectatorHostSession::new);
    signaling
        .msg_tx_mut()
        .take()
        .unwrap()
        .send(SignalingServerMessage::RequestAnswer(offer))
        .unwrap();
    th19.play_sound(th19.sound_manager(), 0x07, 0);
    Some(WaitingForPureP2pSpectator::SignalingCodeRecved {
        signaling,
        session_rx,
        show_hint: false,
        pushed: true,
    })
}

pub enum WaitingForPureP2pSpectator {
    Standby {
        show_hint: bool,
        pushed: bool,
    },
    SignalingCodeRecved {
        signaling: Signaling,
        session_rx: mpsc::Receiver<SpectatorHostSession>,
        show_hint: bool,
        pushed: bool,
    },
    SignalingCodeSent {
        _signaling: Signaling,
        session_rx: mpsc::Receiver<SpectatorHostSession>,
        show_hint: bool,
        pushed: bool,
    },
}

impl WaitingForPureP2pSpectator {
    pub fn standby() -> Self {
        Self::Standby {
            show_hint: false,
            pushed: false,
        }
    }

    fn dummy() -> Self {
        Self::Standby {
            show_hint: false,
            pushed: false,
        }
    }

    fn set_show_hint(&mut self, value: bool) {
        match self {
            Self::Standby { show_hint, .. }
            | Self::SignalingCodeRecved { show_hint, .. }
            | Self::SignalingCodeSent { show_hint, .. } => *show_hint = value,
        }
    }

    fn update_inner(
        &mut self,
        current_pushed: bool,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
    ) -> Result<()> {
        // 観戦者はいつでも受け付ける (合流のタイミングは `SpectatorHostState` が制御する)。
        // `show_hint` は案内表示の有無のみを表し、対戦画面の邪魔にならないようメニュー画面でのみ表示する
        self.set_show_hint(main_menu.is_some());

        match self {
            Self::Standby { pushed, .. } => {
                let prev_pushed = *pushed;
                *pushed = current_pushed;
                if !prev_pushed && current_pushed {
                    if let Some(new_state) = try_start_signaling(th19) {
                        *self = new_state;
                    }
                }
                Ok(())
            }
            Self::SignalingCodeRecved {
                signaling, pushed, ..
            } => {
                let prev_pushed = *pushed;
                *pushed = current_pushed;
                if !prev_pushed && current_pushed {
                    if let Some(new_state) = try_start_signaling(th19) {
                        *self = new_state;
                        return Ok(());
                    }
                }
                signaling.recv();
                let Some(answer) = signaling.answer() else {
                    return Ok(());
                };
                set_clipboard_string(&SignalingCodeType::SpectatorAnswer.to_string(answer))
                    .unwrap();
                th19.play_sound(th19.sound_manager(), 0x57, 0);

                let Self::SignalingCodeRecved {
                    signaling,
                    session_rx,
                    show_hint,
                    pushed,
                } = mem::replace(self, Self::dummy())
                else {
                    unreachable!()
                };
                *self = Self::SignalingCodeSent {
                    _signaling: signaling,
                    session_rx,
                    show_hint,
                    pushed,
                };
                Ok(())
            }
            Self::SignalingCodeSent { pushed, .. } => {
                let prev_pushed = *pushed;
                *pushed = current_pushed;
                if !prev_pushed && current_pushed {
                    if let Some(new_state) = try_start_signaling(th19) {
                        *self = new_state;
                    }
                }
                Ok(())
            }
        }
    }

    pub fn update(&mut self, pushed: bool, menu: Option<&MainMenu>, th19: &Th19) {
        if let Err(err) = self.update_inner(pushed, menu, th19) {
            info!("spectator host error: {:?}", err);
            *self = Self::Standby {
                show_hint: false,
                pushed,
            };
        }
    }
}

pub enum WaitingForSpectator {
    PureP2p(WaitingForPureP2pSpectator),
    ReservedRoom {
        room: SpectatorRelayRoom,
        waiting: WaitingForSpectatorInReservedRoom,
    },
}

impl WaitingForSpectator {
    /// 予約部屋の情報があれば予約部屋で、無ければ Pure P2P で観戦者を待ち受ける
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

    /// 予約部屋の場合、観戦者を受信した後は再度 `new` で待ち受けを作り直す必要がある
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
