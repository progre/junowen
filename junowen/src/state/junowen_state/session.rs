use std::{ffi::c_void, sync::mpsc::RecvError};

use anyhow::Result;
use junowen_lib::{structs::app::MainMenu, structs::settings::GameSettings, Th19};

use crate::{
    file::Features,
    session::{battle::BattleSession, spectator::SpectatorSession},
    signaling::waiting_for_match::WaitingForSpectator,
    state::{
        battle_session_state::BattleSessionState, spectator_session_state::SpectatorSessionState,
    },
};

pub enum Session {
    BattleSession(BattleSessionState),
    SpectatorSession(SpectatorSessionState),
}

impl Session {
    pub fn battle_session(battle_session: BattleSession, waiting: WaitingForSpectator) -> Self {
        Self::BattleSession(BattleSessionState::prepare(battle_session, waiting))
    }

    pub fn spectator_session(session: SpectatorSession) -> Self {
        Self::SpectatorSession(SpectatorSessionState::prepare(session))
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

    pub fn on_render_texts(
        &self,
        features: &[Features],
        th19: &Th19,
        text_renderer: *const c_void,
    ) {
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
