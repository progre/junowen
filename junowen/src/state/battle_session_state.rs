use std::{ffi::c_void, mem, sync::mpsc::RecvError};

use junowen_lib::{GameSettings, Menu, ScreenId, Th19};

use crate::session::battle::BattleSession;

use super::{battle_game::BattleGame, battle_select::BattleSelect, in_session, prepare::Prepare};

pub enum BattleSessionState {
    Null,
    Prepare(Prepare<BattleSession>),
    Select(BattleSelect),
    GameLoading { session: BattleSession },
    Game(BattleGame),
    BackToSelect { session: BattleSession },
}

impl BattleSessionState {
    pub fn game_settings(&self) -> Option<&GameSettings> {
        match self {
            Self::Null => unreachable!(),
            Self::GameLoading { session } | Self::BackToSelect { session } => {
                session.match_initial().map(|x| &x.game_settings)
            }
            Self::Prepare(i) => i.session().match_initial().map(|x| &x.game_settings),
            Self::Select(i) => i.session().match_initial().map(|x| &x.game_settings),
            Self::Game(i) => i.session().match_initial().map(|x| &x.game_settings),
        }
    }

    pub fn inner_battle_session(self) -> BattleSession {
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
        *self = Self::Select(BattleSelect::new(old.inner_battle_session()));
    }
    pub fn change_to_game_loading(&mut self) {
        let old = mem::replace(self, Self::Null);
        *self = Self::GameLoading {
            session: old.inner_battle_session(),
        }
    }
    pub fn change_to_game(&mut self) {
        let old = mem::replace(self, Self::Null);
        *self = Self::Game(BattleGame::new(old.inner_battle_session()));
    }
    pub fn change_to_back_to_select(&mut self) {
        let old = mem::replace(self, Self::Null);
        *self = Self::BackToSelect {
            session: old.inner_battle_session(),
        }
    }

    pub fn update_state(&mut self, th19: &Th19) -> Option<Option<&'static Menu>> {
        match self {
            Self::Null => unreachable!(),
            Self::Prepare(prepare) => {
                let Some(menu) = th19.app().main_loop_tasks().find_menu() else {
                    return Some(None);
                };
                if prepare.update_state(menu, th19) {
                    self.change_to_select();
                }
                Some(Some(menu))
            }
            Self::Select { .. } => {
                let menu = th19.app().main_loop_tasks().find_menu().unwrap();
                match menu.screen_id {
                    ScreenId::GameLoading => {
                        self.change_to_game_loading();
                        Some(Some(menu))
                    }
                    ScreenId::PlayerMatchupSelect => None,
                    _ => Some(Some(menu)),
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
                if th19.round().is_some() {
                    return Some(None);
                }
                self.change_to_back_to_select();
                Some(None)
            }
            Self::BackToSelect { .. } => {
                let Some(menu) = th19.app().main_loop_tasks().find_menu() else {
                    return Some(None);
                };
                if menu.screen_id != ScreenId::CharacterSelect {
                    return Some(Some(menu));
                }
                self.change_to_select();
                Some(Some(menu))
            }
        }
    }

    pub fn update_th19_on_input_players(
        &mut self,
        menu: Option<&Menu>,
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

    pub fn on_input_menu(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        match self {
            Self::Null => unreachable!(),
            Self::Prepare(prepare) => prepare.update_th19_on_input_menu(th19),
            Self::Select(select) => select.update_th19_on_input_menu(th19)?,
            Self::GameLoading { .. } => {}
            Self::Game { .. } => {}
            Self::BackToSelect { .. } => {}
        }
        Ok(())
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
        let (p1, p2) = if session.host() {
            (
                th19.player_name().player_name(),
                session.remote_player_name().into(),
            )
        } else {
            (
                session.remote_player_name().into(),
                th19.player_name().player_name(),
            )
        };
        in_session::on_render_texts(th19, session.delay(), &p1, &p2, text_renderer);
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let Self::Game(game) = self else {
            return Ok(());
        };
        game.on_round_over(th19)
    }
}
