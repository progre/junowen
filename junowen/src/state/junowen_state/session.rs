use std::{ffi::c_void, sync::mpsc::RecvError};

use anyhow::Result;
use junowen_lib::{Th19, structs::app::MainMenu, structs::settings::GameSettings};

use crate::{
    file::Features,
    session::{
        battle::BattleSession as BattleSessionProps,
        spectator::SpectatorSession as SpectatorSessionProps,
    },
    signaling::waiting_for_match::WaitingForSpectator,
    state::{battle_session_state::BattleSession, spectator_session_state::SpectatorSession},
};

pub enum Session {
    BattleSession(BattleSession),
    SpectatorSession(SpectatorSession),
}

impl Session {
    pub fn battle_session(props: BattleSessionProps, waiting: WaitingForSpectator) -> Self {
        Self::BattleSession(BattleSession::prepare(props, waiting))
    }

    pub fn spectator_session(session: SpectatorSessionProps) -> Self {
        Self::SpectatorSession(SpectatorSession::prepare(session))
    }

    pub fn game_settings(&self) -> Option<&GameSettings> {
        match self {
            Self::BattleSession(session_state) => session_state.game_settings(),
            Self::SpectatorSession(session_state) => session_state.game_settings(),
        }
    }

    pub fn update_state(&mut self, th19: &Th19) -> Option<Option<&'static MainMenu>> {
        match self {
            Self::BattleSession(session_state) => session_state.update_state(th19),
            Self::SpectatorSession(session_state) => session_state.update_state(th19),
        }
    }

    pub fn update_th19_on_input_players(
        &mut self,
        menu: Option<&MainMenu>,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        match self {
            Self::BattleSession(session_state) => {
                session_state.update_th19_on_input_players(menu, th19)
            }
            Self::SpectatorSession(session_state) => {
                session_state.update_th19_on_input_players(menu, th19)
            }
        }
    }

    pub fn on_input_menu(&mut self, th19: &mut Th19) -> Result<bool, RecvError> {
        match self {
            Self::BattleSession(session_state) => session_state.on_input_menu(th19).map(|()| true),
            Self::SpectatorSession(session_state) => session_state.on_input_menu(th19),
        }
    }

    pub fn on_render_texts(&self, features: &[Features], th19: &Th19, text_renderer: &c_void) {
        match self {
            Self::BattleSession(session_state) => {
                session_state.on_render_texts(features, th19, text_renderer)
            }
            Self::SpectatorSession(session_state) => {
                session_state.on_render_texts(th19, text_renderer)
            }
        }
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        match self {
            Self::BattleSession(session_state) => session_state.on_round_over(th19),
            Self::SpectatorSession(session_state) => session_state.on_round_over(th19),
        }
    }
}
