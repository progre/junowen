use std::mem;

use anyhow::Result;
use clipboard_win::{get_clipboard_string, set_clipboard_string};
use junowen_lib::{
    connection::signaling::{
        parse_signaling_code, socket::async_read_write_socket::SignalingServerMessage,
        SignalingCodeType,
    },
    Menu, ScreenId, Selection, Th19,
};
use tokio::sync::mpsc::{self, error::TryRecvError};
use tracing::info;

use crate::{
    in_game_lobby::Signaling,
    session::{
        battle::BattleSession,
        spectator::{self, InitialState, SpectatorInitial, SpectatorSessionHost},
        RoundInitial,
    },
};

fn try_start_signaling(th19: &Th19) -> Option<SpectatorHostState> {
    let Ok(ok) = get_clipboard_string() else {
        th19.play_sound(th19.sound_manager(), 0x10, 0);
        return None;
    };
    let Ok((SignalingCodeType::SpectatorOffer, offer)) = parse_signaling_code(&ok) else {
        th19.play_sound(th19.sound_manager(), 0x10, 0);
        return None;
    };
    let (session_tx, session_rx) = mpsc::channel(1);
    let mut signaling = Signaling::new(session_tx, SpectatorSessionHost::new);
    signaling
        .msg_tx_mut()
        .take()
        .unwrap()
        .send(SignalingServerMessage::RequestAnswer(offer))
        .unwrap();
    th19.play_sound(th19.sound_manager(), 0x07, 0);
    Some(SpectatorHostState::SignalingCodeRecved {
        signaling,
        session_rx,
        ready: false,
        pushed: true,
    })
}

fn create_spectator_initial(
    current_screen: ScreenId,
    selection: &Selection,
    battle_session: &BattleSession,
    local_player_name: String,
) -> SpectatorInitial {
    let p1_name = if battle_session.host() {
        local_player_name.to_owned()
    } else {
        battle_session.remote_player_name().clone()
    };
    let p2_name = if battle_session.host() {
        battle_session.remote_player_name().clone()
    } else {
        local_player_name.to_owned()
    };
    SpectatorInitial::new(
        p1_name,
        p2_name,
        battle_session
            .match_initial()
            .as_ref()
            .unwrap()
            .game_settings
            .clone(),
        InitialState::new(
            match current_screen {
                ScreenId::DifficultySelect => spectator::Screen::DifficultySelect,
                ScreenId::CharacterSelect => spectator::Screen::CharacterSelect,
                _ => unreachable!(),
            },
            selection.difficulty as u8,
            selection.p1().character as u8,
            selection.p1().card as u8,
            selection.p2().character as u8,
            selection.p2().card as u8,
        ),
    )
}

pub enum SpectatorHostState {
    Standby {
        ready: bool,
        pushed: bool,
    },
    SignalingCodeRecved {
        signaling: Signaling,
        session_rx: mpsc::Receiver<SpectatorSessionHost>,
        ready: bool,
        pushed: bool,
    },
    SignalingCodeSent {
        signaling: Signaling,
        session_rx: mpsc::Receiver<SpectatorSessionHost>,
        ready: bool,
        pushed: bool,
    },
    Connected {
        session: SpectatorSessionHost,
        pushed: bool,
    },
}

impl SpectatorHostState {
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
            Self::Connected { .. } => unreachable!(),
        }
    }

    pub fn send_init_round_if_connected(&mut self, th19: &Th19) {
        let Self::Connected {
            session,
            pushed: current_pushed,
            ..
        } = self
        else {
            return;
        };
        if let Err(err) = session.send_init_round(RoundInitial {
            seed1: th19.rand_seed1().unwrap(),
            seed2: th19.rand_seed2().unwrap(),
            seed3: th19.rand_seed3().unwrap(),
            seed4: th19.rand_seed4().unwrap(),
        }) {
            info!("spectator host error: {:?}", err);
            *self = Self::Standby {
                ready: true,
                pushed: *current_pushed,
            };
        }
    }

    fn update_inner(
        &mut self,
        current_pushed: bool,
        menu: Option<&Menu>,
        th19: &Th19,
        battle_session: &BattleSession,
        p1_input: u16,
        p2_input: u16,
    ) -> Result<()> {
        let selection = th19.selection();
        if !matches!(self, Self::Connected { .. }) {
            self.set_ready(
                menu.is_some()
                    && menu.unwrap().screen_id == ScreenId::DifficultySelect
                    && selection.p1().card == 0
                    && selection.p2().card == 0,
            );
        }

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
            Self::SignalingCodeSent {
                session_rx,
                ready,
                pushed,
                ..
            } => {
                let prev_pushed = *pushed;
                *pushed = current_pushed;
                if !prev_pushed && current_pushed {
                    if let Some(new_state) = try_start_signaling(th19) {
                        *self = new_state;
                        return Ok(());
                    }
                }
                match session_rx.try_recv() {
                    Ok(session) => {
                        let Some(menu) = menu else {
                            info!("spectator not supported yet.");
                            *self = Self::Standby {
                                ready: *ready,
                                pushed: *pushed,
                            };
                            return Ok(());
                        };
                        if menu.screen_id != ScreenId::DifficultySelect
                            || selection.p1().card != 0
                            || selection.p2().card != 0
                        {
                            info!("spectator not supported yet.");
                            *self = Self::Standby {
                                ready: *ready,
                                pushed: *pushed,
                            };
                            return Ok(());
                        }
                        session.send_init_spectator(create_spectator_initial(
                            menu.screen_id,
                            selection,
                            battle_session,
                            th19.player_name().player_name().to_string(),
                        ))?;
                        session.send_init_round(RoundInitial {
                            seed1: th19.rand_seed1().unwrap(),
                            seed2: th19.rand_seed2().unwrap(),
                            seed3: th19.rand_seed3().unwrap(),
                            seed4: th19.rand_seed4().unwrap(),
                        })?;
                        session.send_inputs(p1_input, p2_input)?;
                        *self = Self::Connected {
                            session,
                            pushed: *pushed,
                        };
                        Ok(())
                    }
                    Err(TryRecvError::Empty) => Ok(()),
                    Err(TryRecvError::Disconnected) => {
                        th19.play_sound(th19.sound_manager(), 0x10, 0);
                        *self = Self::Standby {
                            ready: *ready,
                            pushed: *pushed,
                        };
                        Ok(())
                    }
                }
            }
            Self::Connected {
                session, pushed, ..
            } => {
                let prev_pushed = *pushed;
                *pushed = current_pushed;
                if !prev_pushed && current_pushed {
                    if let Some(new_state) = try_start_signaling(th19) {
                        *self = new_state;
                        return Ok(());
                    }
                }
                session.send_inputs(p1_input, p2_input)?;
                Ok(())
            }
        }
    }

    pub fn update(
        &mut self,
        pushed: bool,
        menu: Option<&Menu>,
        th19: &Th19,
        battle_session: &BattleSession,
        p1_input: u16,
        p2_input: u16,
    ) {
        if let Err(err) = self.update_inner(pushed, menu, th19, battle_session, p1_input, p2_input)
        {
            info!("spectator host error: {:?}", err);
            *self = Self::Standby {
                ready: false,
                pushed,
            };
        }
    }
}
