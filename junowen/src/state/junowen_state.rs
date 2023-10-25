use std::{ffi::c_void, mem, sync::mpsc::RecvError};

use junowen_lib::{
    Fn011560, Fn0b7d40, Fn0d5ae0, Fn10f720, Menu, RenderingText, ScreenId, Selection, Th19,
};
use tokio::sync::mpsc::Receiver;
use tracing::trace;

use crate::{
    in_game_lobby::{Lobby, TitleMenuModifier},
    session::BattleSession,
};

use super::{game::Game, in_session, prepare::Prepare, select::Select, standby};

pub enum JunowenState {
    Standby,
    Prepare(Prepare),
    Select(Select),
    GameLoading { battle_session: BattleSession },
    Game(Game),
    BackToSelect { battle_session: BattleSession },
}

impl JunowenState {
    pub fn session(&self) -> Option<&BattleSession> {
        match self {
            JunowenState::Standby => None,
            JunowenState::GameLoading { battle_session }
            | JunowenState::BackToSelect { battle_session } => Some(battle_session),
            JunowenState::Prepare(inner) => Some(inner.battle_session()),
            JunowenState::Select(inner) => Some(inner.battle_session()),
            JunowenState::Game(inner) => Some(inner.battle_session()),
        }
    }

    pub fn inner_session(self) -> BattleSession {
        match self {
            JunowenState::Standby => unreachable!(),
            JunowenState::GameLoading { battle_session }
            | JunowenState::BackToSelect { battle_session } => battle_session,
            JunowenState::Prepare(inner) => inner.inner_battle_session(),
            JunowenState::Select(inner) => inner.inner_battle_session(),
            JunowenState::Game(inner) => inner.inner_battle_session(),
        }
    }

    pub fn start_session(&mut self, battle_session: BattleSession) {
        *self = JunowenState::Prepare(Prepare::new(battle_session, 0));
    }

    pub fn change_to_prepare(&mut self, new_state: u8) {
        let JunowenState::Prepare(prepare) = self else {
            unreachable!();
        };
        prepare.set_state(new_state);
    }

    pub fn change_to_select(&mut self) {
        let old = mem::replace(self, JunowenState::Standby);
        *self = JunowenState::Select(Select::new(old.inner_session()));
    }
    pub fn change_to_game_loading(&mut self) {
        let old = mem::replace(self, JunowenState::Standby);
        *self = JunowenState::GameLoading {
            battle_session: old.inner_session(),
        }
    }
    pub fn change_to_game(&mut self) {
        let old = mem::replace(self, JunowenState::Standby);
        *self = JunowenState::Game(Game::new(old.inner_session()));
    }
    pub fn change_to_back_to_select(&mut self) {
        let old = mem::replace(self, JunowenState::Standby);
        *self = JunowenState::BackToSelect {
            battle_session: old.inner_session(),
        }
    }
    pub fn end_session(&mut self) {
        *self = JunowenState::Standby;
    }

    fn update_state(
        &mut self,
        th19: &Th19,
        battle_session_rx: &mut Receiver<BattleSession>,
    ) -> Option<Option<&'static Menu>> {
        match &self {
            JunowenState::Standby => {
                if let Ok(session) = battle_session_rx.try_recv() {
                    trace!("session received");
                    self.start_session(session);
                    return Some(None);
                };
                None
            }
            JunowenState::Prepare(prepare) => {
                let Some(menu) = th19.app().main_loop_tasks().find_menu() else {
                    return Some(None);
                };
                match prepare.state() {
                    0 => {
                        if menu.screen_id != ScreenId::Title {
                            return Some(Some(menu));
                        }
                        let new_state = if th19.input_devices().is_conflict_input_device() {
                            1
                        } else {
                            2
                        };
                        self.change_to_prepare(new_state);
                        Some(Some(menu))
                    }
                    1 => {
                        if th19.input_devices().is_conflict_input_device() {
                            return Some(Some(menu));
                        }
                        self.change_to_prepare(0);
                        Some(Some(menu))
                    }
                    2 => {
                        if menu.screen_id != ScreenId::DifficultySelect {
                            return Some(Some(menu));
                        }
                        self.change_to_select();
                        Some(Some(menu))
                    }
                    _ => unreachable!(),
                }
            }
            JunowenState::Select { .. } => {
                let menu = th19.app().main_loop_tasks().find_menu().unwrap();
                match menu.screen_id {
                    ScreenId::GameLoading => {
                        self.change_to_game_loading();
                        Some(Some(menu))
                    }
                    ScreenId::PlayerMatchupSelect => {
                        self.end_session();
                        None
                    }
                    _ => Some(Some(menu)),
                }
            }
            JunowenState::GameLoading { .. } => {
                let Some(game) = th19.round() else {
                    return Some(None);
                };
                if !game.is_first_frame() {
                    return Some(None);
                }
                self.change_to_game();
                Some(None)
            }
            JunowenState::Game { .. } => {
                if th19.round().is_some() {
                    return Some(None);
                }
                self.change_to_back_to_select();
                Some(None)
            }
            JunowenState::BackToSelect { .. } => {
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

    fn update_th19_on_input_players(
        &mut self,
        menu: Option<&Menu>,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        match self {
            JunowenState::Standby => unreachable!(),
            JunowenState::Prepare(prepare) => prepare.update_th19_on_input_players(th19),
            JunowenState::Select(select) => {
                select.update_th19_on_input_players(menu.unwrap(), th19)?
            }
            JunowenState::GameLoading { .. } => {}
            JunowenState::Game(game) => game.update_th19(th19)?,
            JunowenState::BackToSelect { .. } => {}
        }
        Ok(())
    }

    pub fn on_input_players(
        &mut self,
        th19: &mut Th19,
        battle_session_rx: &mut Receiver<BattleSession>,
    ) -> Result<(), RecvError> {
        let Some(menu) = self.update_state(th19, battle_session_rx) else {
            return Ok(());
        };
        self.update_th19_on_input_players(menu, th19)
    }

    pub fn on_input_menu(
        &mut self,
        th19: &mut Th19,
        title_menu_modifier: &mut TitleMenuModifier,
        lobby: &mut Lobby,
    ) -> Result<(), RecvError> {
        match self {
            JunowenState::Standby => {
                standby::update_th19_on_input_menu(th19, title_menu_modifier, lobby);
            }
            JunowenState::Prepare(prepare) => prepare.update_th19_on_input_menu(th19),
            JunowenState::Select(select) => select.update_th19_on_input_menu(th19)?,
            JunowenState::GameLoading { .. } => {}
            JunowenState::Game { .. } => {}
            JunowenState::BackToSelect { .. } => {}
        }
        Ok(())
    }

    pub fn render_object(
        &self,
        title_menu_modifier: &TitleMenuModifier,
        old: Fn0b7d40,
        obj_renderer: *const c_void,
        obj: *const c_void,
    ) {
        if self.session().is_none() {
            standby::render_object(title_menu_modifier, old, obj_renderer, obj);
            return;
        }
        old(obj_renderer, obj);
    }

    pub fn render_text(
        &self,
        th19: &Th19,
        title_menu_modifier: &TitleMenuModifier,
        old: Fn0d5ae0,
        text_renderer: *const c_void,
        text: &mut RenderingText,
    ) -> u32 {
        if self.session().is_none() {
            return standby::render_text(th19, title_menu_modifier, old, text_renderer, text);
        }
        old(text_renderer, text)
    }

    pub fn on_render_texts(
        &self,
        th19: &Th19,
        title_menu_modifier: &TitleMenuModifier,
        lobby: &Lobby,
        text_renderer: *const c_void,
    ) {
        let Some(session) = self.session() else {
            standby::on_render_texts(th19, title_menu_modifier, lobby, text_renderer);
            return;
        };
        in_session::on_render_texts(th19, session, text_renderer);
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let JunowenState::Game(game) = self else {
            return Ok(());
        };
        game.on_round_over(th19)
    }

    pub fn is_online_vs(&self, this: *const Selection, old: Fn011560) -> u8 {
        let ret = old(this);
        if self.session().is_some() {
            return 1;
        }
        ret
    }

    pub fn on_rewrite_controller_assignments(
        &self,
        th19: &mut Th19,
        old_fn: fn(&mut Th19) -> Fn10f720,
    ) {
        if self.session().is_none() {
            old_fn(th19)();
            return;
        }
        in_session::on_rewrite_controller_assignments(th19, old_fn);
    }

    pub fn on_loaded_game_settings(&self, th19: &mut Th19) {
        if let Some(match_initial) = &self.session().and_then(|x| x.match_initial().as_ref()) {
            th19.put_game_settings_in_game(&match_initial.game_settings)
                .unwrap();
        }
    }
}
