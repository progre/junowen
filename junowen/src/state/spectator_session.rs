mod in_session;
mod spectator_game;
mod spectator_select;

use std::{ffi::c_void, sync::mpsc::RecvError};

use anyhow::Result;
use junowen_lib::{
    Th19,
    structs::settings::GameSettings,
    structs::{
        app::{MainMenu, ScreenId},
        input_devices::InputFlags,
    },
};

use crate::session::spectator::SpectatorSession as SpectatorSessionProps;

use super::prepare::Prepare;

use {spectator_game::SpectatorGame, spectator_select::SpectatorSelect};

pub struct SpectatorSession {
    props: SpectatorSessionProps,
    state: SpectatorSessionState,
}

enum SpectatorSessionState {
    Prepare(Prepare),
    Select(SpectatorSelect),
    GameLoading,
    Game(SpectatorGame),
    BackToSelect,
}

impl SpectatorSession {
    pub fn prepare(props: SpectatorSessionProps) -> Self {
        Self {
            props,
            state: SpectatorSessionState::Prepare(Prepare::new()),
        }
    }

    pub fn game_settings(&self) -> Option<&GameSettings> {
        self.props.spectator_initial().map(|x| x.game_settings())
    }

    pub fn change_to_select(&mut self) {
        self.state = SpectatorSessionState::Select(SpectatorSelect::new());
    }
    pub fn change_to_game_loading(&mut self) {
        self.state = SpectatorSessionState::GameLoading;
    }
    pub fn change_to_game(&mut self) {
        self.state = SpectatorSessionState::Game(SpectatorGame);
    }
    pub fn change_to_back_to_select(&mut self) {
        self.state = SpectatorSessionState::BackToSelect;
    }

    pub fn update_state(&mut self, th19: &Th19) -> Option<Option<&'static MainMenu>> {
        match &mut self.state {
            SpectatorSessionState::Prepare(prepare) => {
                let Some(main_menu) = th19.app().main_loop_tasks().find_main_menu() else {
                    return Some(None);
                };
                if prepare.update_state(main_menu, th19) {
                    self.change_to_select();
                }
                Some(Some(main_menu))
            }
            SpectatorSessionState::Select { .. } => {
                let main_menu = th19.app().main_loop_tasks().find_main_menu().unwrap();
                match main_menu.screen_id() {
                    ScreenId::PlayerMatchupSelect => None,
                    ScreenId::CharacterSelect => {
                        if th19.input_devices().p1_input().current().0 & InputFlags::PAUSE != None {
                            return None;
                        }
                        Some(Some(main_menu))
                    }
                    ScreenId::GameLoading => {
                        self.change_to_game_loading();
                        Some(Some(main_menu))
                    }
                    _ => Some(Some(main_menu)),
                }
            }
            SpectatorSessionState::GameLoading { .. } => {
                let Some(round_frame) = th19.round_frame() else {
                    return Some(None);
                };
                if !round_frame.is_first_frame() {
                    return Some(None);
                }
                self.change_to_game();
                Some(None)
            }
            SpectatorSessionState::Game { .. } => {
                if th19.input_devices().p1_input().current().0 & InputFlags::PAUSE != None {
                    return None;
                }
                if th19.round_frame().is_some() {
                    return Some(None);
                }
                self.change_to_back_to_select();
                Some(None)
            }
            SpectatorSessionState::BackToSelect { .. } => {
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
            SpectatorSessionState::Prepare(prepare) => prepare.update_th19_on_input_players(th19),
            SpectatorSessionState::Select(select) => {
                select.update_th19_on_input_players(&mut self.props, menu.unwrap(), th19)?
            }
            SpectatorSessionState::GameLoading { .. } => {
                if th19.no_wait() {
                    th19.set_no_wait(false);
                }
            }
            SpectatorSessionState::Game(game) => game.update_th19(&mut self.props, th19)?,
            SpectatorSessionState::BackToSelect { .. } => {}
        }
        Ok(())
    }

    pub fn on_input_menu(&mut self, th19: &mut Th19) -> Result<bool, RecvError> {
        match &mut self.state {
            SpectatorSessionState::Prepare(prepare) => prepare.update_th19_on_input_menu(th19),
            SpectatorSessionState::Select(select) => {
                let main_menu = th19
                    .app_mut()
                    .main_loop_tasks_mut()
                    .find_main_menu_mut()
                    .unwrap();
                if main_menu.screen_id() == ScreenId::DifficultySelect
                    && th19.menu_input().current().0 & InputFlags::PAUSE != None
                {
                    return Ok(false);
                }
                select.update_th19_on_input_menu(&mut self.props, main_menu, th19)?;
            }
            SpectatorSessionState::GameLoading { .. } => {}
            SpectatorSessionState::Game { .. } => {}
            SpectatorSessionState::BackToSelect { .. } => {}
        }
        Ok(true)
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: &c_void) {
        let Some(initial) = self.props.spectator_initial() else {
            return;
        };
        in_session::on_render_texts_spectator(
            th19,
            text_renderer,
            initial.p1_name(),
            initial.p2_name(),
        );
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let SpectatorSessionState::Game(game) = &mut self.state else {
            return Ok(());
        };
        game.on_round_over(&mut self.props, th19)
    }
}
