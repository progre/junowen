mod in_session;
mod spectator_game;
mod spectator_select;

use std::{ffi::c_void, mem, sync::mpsc::RecvError};

use anyhow::Result;
use junowen_lib::{GameSettings, InputFlags, MainMenu, ScreenId, Th19};

use crate::session::spectator::SpectatorSession;

use super::prepare::Prepare;

use {spectator_game::SpectatorGame, spectator_select::SpectatorSelect};

pub enum SpectatorSessionState {
    Null,
    Prepare(Prepare<SpectatorSession>),
    Select(SpectatorSelect),
    GameLoading { session: SpectatorSession },
    Game(SpectatorGame),
    BackToSelect { session: SpectatorSession },
}

impl SpectatorSessionState {
    pub fn prepare(session: SpectatorSession) -> Self {
        Self::Prepare(Prepare::new(session))
    }

    pub fn game_settings(&self) -> Option<&GameSettings> {
        let init = match self {
            Self::Null => unreachable!(),
            Self::GameLoading { session } | Self::BackToSelect { session } => {
                session.spectator_initial()?
            }
            Self::Prepare(i) => i.session().spectator_initial()?,
            Self::Select(i) => i.session().spectator_initial()?,
            Self::Game(i) => i.session().spectator_initial()?,
        };
        Some(init.game_settings())
    }

    pub fn inner_spectator_session(self) -> SpectatorSession {
        match self {
            Self::Null => unreachable!(),
            Self::GameLoading { session } | Self::BackToSelect { session } => session,
            Self::Prepare(inner) => inner.inner_session(),
            Self::Select(inner) => inner.inner_session(),
            Self::Game(inner) => inner.inner_session(),
        }
    }

    pub fn change_to_select(&mut self) {
        let old = mem::replace(self, Self::Null);
        *self = Self::Select(SpectatorSelect::new(old.inner_spectator_session()));
    }
    pub fn change_to_game_loading(&mut self) {
        let old = mem::replace(self, Self::Null);
        *self = Self::GameLoading {
            session: old.inner_spectator_session(),
        }
    }
    pub fn change_to_game(&mut self) {
        let old = mem::replace(self, Self::Null);
        *self = Self::Game(SpectatorGame::new(old.inner_spectator_session()));
    }
    pub fn change_to_back_to_select(&mut self) {
        let old = mem::replace(self, Self::Null);
        *self = Self::BackToSelect {
            session: old.inner_spectator_session(),
        }
    }

    pub fn update_state(&mut self, th19: &Th19) -> Option<Option<&'static MainMenu>> {
        match self {
            Self::Null => unreachable!(),
            Self::Prepare(prepare) => {
                let Some(main_menu) = th19.app().main_loop_tasks().find_main_menu() else {
                    return Some(None);
                };
                if prepare.update_state(main_menu, th19) {
                    self.change_to_select();
                }
                Some(Some(main_menu))
            }
            Self::Select { .. } => {
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
            Self::GameLoading { .. } => {
                let Some(game) = th19.round() else {
                    return Some(None);
                };
                if !game.is_first_frame() {
                    return Some(None);
                }
                self.change_to_game();
                Some(None)
            }
            Self::Game { .. } => {
                if th19.input_devices().p1_input().current().0 & InputFlags::PAUSE != None {
                    return None;
                }
                if th19.round().is_some() {
                    return Some(None);
                }
                self.change_to_back_to_select();
                Some(None)
            }
            Self::BackToSelect { .. } => {
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
        match self {
            Self::Null => unreachable!(),
            Self::Prepare(prepare) => prepare.update_th19_on_input_players(th19),
            Self::Select(select) => select.update_th19_on_input_players(menu.unwrap(), th19)?,
            Self::GameLoading { .. } => {}
            Self::Game(game) => game.update_th19(th19)?,
            Self::BackToSelect { .. } => {}
        }
        Ok(())
    }

    pub fn on_input_menu(&mut self, th19: &mut Th19) -> Result<bool, RecvError> {
        match self {
            Self::Null => unreachable!(),
            Self::Prepare(prepare) => prepare.update_th19_on_input_menu(th19),
            Self::Select(select) => {
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
                select.update_th19_on_input_menu(main_menu, th19)?;
            }
            Self::GameLoading { .. } => {}
            Self::Game { .. } => {}
            Self::BackToSelect { .. } => {}
        }
        Ok(true)
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        let session = {
            match self {
                Self::Null => unreachable!(),
                Self::GameLoading { session } | Self::BackToSelect { session } => session,
                Self::Prepare(inner) => inner.session(),
                Self::Select(inner) => inner.session(),
                Self::Game(inner) => inner.session(),
            }
        };
        let Some(initial) = session.spectator_initial() else {
            return;
        };
        in_session::on_render_texts_spectator(
            th19,
            initial.p1_name(),
            initial.p2_name(),
            text_renderer,
        );
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let Self::Game(game) = self else {
            return Ok(());
        };
        game.on_round_over(th19)
    }
}
