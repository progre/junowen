use std::mem;

use anyhow::Result;
use clipboard_win::{get_clipboard_string, set_clipboard_string};
use junowen_lib::{
    connection::signaling::{
        parse_signaling_code, socket::async_read_write_socket::SignalingServerMessage,
        SignalingCodeType,
    },
    Menu, ScreenId, Th19,
};
use tokio::sync::mpsc::{self, error::TryRecvError};
use tracing::info;

use crate::{in_game_lobby::Signaling, session::spectator_host::SpectatorHostSession};

use super::waiting_for_match::rooms::WaitingForSpectatorInReservedRoom;

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
        ready: false,
        pushed: true,
    })
}

pub enum WaitingForPureP2pSpectator {
    Standby {
        ready: bool,
        pushed: bool,
    },
    SignalingCodeRecved {
        signaling: Signaling,
        session_rx: mpsc::Receiver<SpectatorHostSession>,
        ready: bool,
        pushed: bool,
    },
    SignalingCodeSent {
        signaling: Signaling,
        session_rx: mpsc::Receiver<SpectatorHostSession>,
        ready: bool,
        pushed: bool,
    },
}

impl WaitingForPureP2pSpectator {
    pub fn standby() -> Self {
        Self::Standby {
            ready: false,
            pushed: false,
        }
    }

    fn dummy() -> Self {
        Self::Standby {
            ready: false,
            pushed: false,
        }
    }

    fn set_ready(&mut self, value: bool) {
        match self {
            Self::Standby { ready, .. }
            | Self::SignalingCodeRecved { ready, .. }
            | Self::SignalingCodeSent { ready, .. } => *ready = value,
        }
    }

    fn update_inner(
        &mut self,
        current_pushed: bool,
        menu: Option<&Menu>,
        th19: &Th19,
    ) -> Result<()> {
        let selection = th19.selection();
        self.set_ready(
            menu.is_some()
                && menu.unwrap().screen_id == ScreenId::DifficultySelect
                && selection.p1().card == 0
                && selection.p2().card == 0,
        );

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
                    ready,
                    pushed,
                } = mem::replace(self, Self::dummy())
                else {
                    unreachable!()
                };
                *self = Self::SignalingCodeSent {
                    signaling,
                    session_rx,
                    ready,
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

    pub fn update(&mut self, pushed: bool, menu: Option<&Menu>, th19: &Th19) {
        if let Err(err) = self.update_inner(pushed, menu, th19) {
            info!("spectator host error: {:?}", err);
            *self = Self::Standby {
                ready: false,
                pushed,
            };
        }
    }
}

pub enum WaitingForSpectator {
    PureP2p(WaitingForPureP2pSpectator),
    ReservedRoom(WaitingForSpectatorInReservedRoom),
}

impl WaitingForSpectator {
    pub fn try_recv_session(
        &mut self,
        pushed: bool,
        menu: Option<&Menu>,
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
            Self::ReservedRoom(waiting) => match waiting.try_session_and_waiting_for_spectator() {
                Ok((session, waiting)) => {
                    *self = waiting;
                    Some(session)
                }
                Err(_) => None,
            },
        }
    }
}
