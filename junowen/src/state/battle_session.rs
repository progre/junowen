mod battle_game;
mod battle_select;
mod in_session;
mod spectator_host;
mod utils;

use std::{ffi::c_void, sync::mpsc::RecvError};

use anyhow::Result;
use junowen_lib::{
    Th19,
    structs::app::{MainMenu, ScreenId},
    structs::settings::GameSettings,
};

use crate::{
    file::Features, session::battle::BattleSession as BattleSessionProps,
    signaling::waiting_for_match::WaitingForSpectator,
};

use self::in_session::RenderingStatus;

use super::prepare::Prepare;

use {battle_game::BattleGame, battle_select::BattleSelect, spectator_host::SpectatorHostState};

pub struct BattleSession {
    props: BattleSessionProps,
    spectator_host_state: SpectatorHostState,
    state: BattleSessionStateState,
}

enum BattleSessionStateState {
    Prepare(Prepare),
    Select(BattleSelect),
    GameLoading,
    Game(BattleGame),
    BackToSelect,
}

impl BattleSession {
    pub fn prepare(props: BattleSessionProps, waiting: WaitingForSpectator) -> Self {
        Self {
            props,
            spectator_host_state: SpectatorHostState::new(waiting),
            state: BattleSessionStateState::Prepare(Prepare::new()),
        }
    }

    pub fn game_settings(&self) -> Option<&GameSettings> {
        self.props.match_initial().map(|x| &x.game_settings)
    }

    pub fn change_to_select(&mut self) {
        self.state = BattleSessionStateState::Select(BattleSelect::new());
    }
    pub fn change_to_game_loading(&mut self) {
        self.state = BattleSessionStateState::GameLoading;
    }
    pub fn change_to_game(&mut self) {
        self.state = BattleSessionStateState::Game(BattleGame);
    }
    pub fn change_to_back_to_select(&mut self) {
        self.state = BattleSessionStateState::BackToSelect;
    }

    pub fn update_state(&mut self, th19: &Th19) -> Option<Option<&'static MainMenu>> {
        match &mut self.state {
            BattleSessionStateState::Prepare(prepare) => {
                let Some(main_menu) = th19.app().main_loop_tasks().find_main_menu() else {
                    return Some(None);
                };
                if prepare.update_state(main_menu, th19) {
                    self.change_to_select();
                }
                Some(Some(main_menu))
            }
            BattleSessionStateState::Select { .. } => {
                let main_menu = th19.app().main_loop_tasks().find_main_menu().unwrap();
                match main_menu.screen_id() {
                    ScreenId::GameLoading => {
                        self.change_to_game_loading();
                        Some(Some(main_menu))
                    }
                    ScreenId::PlayerMatchupSelect => None,
                    _ => Some(Some(main_menu)),
                }
            }
            BattleSessionStateState::GameLoading { .. } => {
                let Some(round_frame) = th19.round_frame() else {
                    return Some(None);
                };
                if !round_frame.is_first_frame() {
                    return Some(None);
                }
                self.change_to_game();
                Some(None)
            }
            BattleSessionStateState::Game { .. } => {
                if th19.round_frame().is_some() {
                    return Some(None);
                }
                self.change_to_back_to_select();
                Some(None)
            }
            BattleSessionStateState::BackToSelect { .. } => {
                let Some(main_menu) = th19.app().main_loop_tasks().find_main_menu() else {
                    return Some(None);
                };
                if main_menu.screen_id() != ScreenId::CharacterSelect {
                    return Some(Some(main_menu));
                }
                self.change_to_select();
                Some(Some(main_menu))
            }
        }
    }

    pub fn update_th19_on_input_players(
        &mut self,
        menu: Option<&MainMenu>,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        match &mut self.state {
            BattleSessionStateState::Prepare(prepare) => prepare.update_th19_on_input_players(th19),
            BattleSessionStateState::Select(select) => select.update_th19_on_input_players(
                &mut self.props,
                &mut self.spectator_host_state,
                menu.unwrap(),
                th19,
            )?,
            BattleSessionStateState::GameLoading { .. } => {}
            BattleSessionStateState::Game(game) => {
                game.update_th19(&mut self.props, &mut self.spectator_host_state, th19)?
            }
            BattleSessionStateState::BackToSelect { .. } => {}
        }
        Ok(())
    }

    pub fn on_input_menu(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        match &mut self.state {
            BattleSessionStateState::Prepare(prepare) => prepare.update_th19_on_input_menu(th19),
            BattleSessionStateState::Select(select) => select.update_th19_on_input_menu(
                &mut self.props,
                &mut self.spectator_host_state,
                th19,
            )?,
            BattleSessionStateState::GameLoading { .. } => {}
            BattleSessionStateState::Game { .. } => {}
            BattleSessionStateState::BackToSelect { .. } => {}
        }
        Ok(())
    }

    pub fn on_render_texts(&self, features: &[Features], th19: &Th19, text_renderer: &c_void) {
        let (p1_name, p2_name) = if self.props.host() {
            (
                th19.vs_mode().player_name(),
                self.props.remote_player_name().as_str(),
            )
        } else {
            (
                self.props.remote_player_name().as_str(),
                th19.vs_mode().player_name(),
            )
        };

        let game_settings = 'ret: {
            if !features.contains(&Features::ShowSettings) {
                break 'ret None;
            }
            if !matches!(self.state, BattleSessionStateState::Select { .. }) {
                break 'ret None;
            }
            self.props.match_initial().map(|x| &x.game_settings)
        };
        let status = RenderingStatus {
            host: self.props.host(),
            delay: self.props.delay(),
            p1_name,
            p2_name,
            game_settings,
            spectator_host_state: &self.spectator_host_state,
        };
        in_session::on_render_texts(th19, text_renderer, status);
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let BattleSessionStateState::Game(game) = &mut self.state else {
            return Ok(());
        };
        game.on_round_over(&mut self.props, &mut self.spectator_host_state, th19)
    }
}
