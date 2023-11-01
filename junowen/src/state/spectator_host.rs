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
    Some(SpectatorHostState::SignalingCodeRecved(
        signaling, session_rx, true,
    ))
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
    /// pushed: bool
    Standby(bool),
    /// pushed: bool
    SignalingCodeRecved(Signaling, mpsc::Receiver<SpectatorSessionHost>, bool),
    /// pushed: bool
    SignalingCodeSent(Signaling, mpsc::Receiver<SpectatorSessionHost>, bool),
    /// pushed: bool
    Connected(SpectatorSessionHost, bool),
}

impl SpectatorHostState {
    pub fn send_init_round_if_connected(&mut self, th19: &Th19) {
        let Self::Connected(session, current_pushed) = self else {
            return;
        };
        if let Err(err) = session.send_init_round(RoundInitial {
            seed1: th19.rand_seed1().unwrap(),
            seed2: th19.rand_seed2().unwrap(),
            seed3: th19.rand_seed3().unwrap(),
            seed4: th19.rand_seed4().unwrap(),
        }) {
            info!("spectator host error: {:?}", err);
            *self = Self::Standby(*current_pushed);
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
        match self {
            Self::Standby(pushed) => {
                let prev_pushed = *pushed;
                *pushed = current_pushed;
                if !prev_pushed && current_pushed {
                    if let Some(new_state) = try_start_signaling(th19) {
                        *self = new_state;
                    }
                }
                Ok(())
            }
            Self::SignalingCodeRecved(signaling, _session_rx, pushed) => {
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

                let Self::SignalingCodeRecved(signaling, session_rx, pushed) =
                    mem::replace(self, Self::Standby(false))
                else {
                    unreachable!()
                };
                *self = Self::SignalingCodeSent(signaling, session_rx, pushed);
                Ok(())
            }
            Self::SignalingCodeSent(_signaling, session_rx, pushed) => {
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
                            *self = Self::Standby(*pushed);
                            return Ok(());
                        };
                        let selection = th19.selection();
                        if menu.screen_id != ScreenId::DifficultySelect
                            || selection.p1().card != 0
                            || selection.p2().card != 0
                        {
                            info!("spectator not supported yet.");
                            *self = Self::Standby(*pushed);
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
                        *self = Self::Connected(session, *pushed);
                        Ok(())
                    }
                    Err(TryRecvError::Empty) => Ok(()),
                    Err(TryRecvError::Disconnected) => {
                        th19.play_sound(th19.sound_manager(), 0x10, 0);
                        *self = Self::Standby(*pushed);
                        Ok(())
                    }
                }
            }
            Self::Connected(session, pushed) => {
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
        current_pushed: bool,
        menu: Option<&Menu>,
        th19: &Th19,
        battle_session: &BattleSession,
        p1_input: u16,
        p2_input: u16,
    ) {
        if let Err(err) = self.update_inner(
            current_pushed,
            menu,
            th19,
            battle_session,
            p1_input,
            p2_input,
        ) {
            info!("spectator host error: {:?}", err);
            *self = Self::Standby(current_pushed);
        }
    }
}
