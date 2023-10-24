mod game;
mod in_session;
mod junowen_state;
mod prepare;
mod select;
mod standby;

use std::{ffi::c_void, sync::mpsc::RecvError};

use anyhow::Result;
use getset::{Getters, MutGetters};
use junowen_lib::{
    Fn011560, Fn0b7d40, Fn0d5ae0, Fn10f720, Menu, RenderingText, ScreenId, Selection, Th19,
};
use tokio::sync::mpsc;
use tracing::{debug, trace};

use self::junowen_state::JunowenState;
use crate::{
    in_game_lobby::{Lobby, TitleMenuModifier},
    session::BattleSession,
};

#[derive(Getters, MutGetters)]
pub struct State {
    battle_session_rx: mpsc::Receiver<BattleSession>,
    #[getset(get = "pub", get_mut = "pub")]
    th19: Th19,
    title_menu_modifier: TitleMenuModifier,
    lobby: Lobby,
    junowen_state: JunowenState,
}

impl State {
    pub fn new(th19: Th19) -> Self {
        let (battle_session_tx, battle_session_rx) = mpsc::channel(1);
        Self {
            battle_session_rx,
            th19,
            title_menu_modifier: TitleMenuModifier::new(),
            lobby: Lobby::new(battle_session_tx),
            junowen_state: JunowenState::Standby,
        }
    }

    fn end_session(&mut self) {
        self.junowen_state.end_session();
        self.lobby.reset_depth();
    }

    fn update_state(&mut self) -> Option<(bool, Option<&'static Menu>)> {
        match self.junowen_state {
            JunowenState::Standby => {
                if let Ok(session) = self.battle_session_rx.try_recv() {
                    trace!("session received");
                    self.junowen_state.start_session(session);
                    return Some((true, None));
                };
                None
            }
            JunowenState::Prepare {
                state: prepare_state,
                ..
            } => prepare::update_state(self, prepare_state),
            JunowenState::Select { .. } => select::update_state(self),
            JunowenState::GameLoading { .. } => {
                let Some(game) = self.th19.round() else {
                    return Some((false, None));
                };
                if !game.is_first_frame() {
                    return Some((false, None));
                }
                self.junowen_state.change_to_game();
                Some((true, None))
            }
            JunowenState::Game { .. } => game::update_state(self),
            JunowenState::BackToSelect { .. } => {
                let Some(menu) = self.th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
                    return Some((false, None));
                };
                if menu.screen_id != ScreenId::CharacterSelect {
                    return Some((false, Some(menu)));
                }
                self.junowen_state.change_to_select();
                Some((true, Some(menu)))
            }
        }
    }

    fn update_th19_on_input_players(
        &mut self,
        changed: bool,
        menu: Option<&Menu>,
    ) -> Result<(), RecvError> {
        match &mut self.junowen_state {
            JunowenState::Standby => unreachable!(),
            JunowenState::Prepare {
                state: prepare_state,
                ..
            } => {
                prepare::update_th19_on_input_players(&mut self.th19, *prepare_state);
                Ok(())
            }
            JunowenState::Select { battle_session } => select::update_th19_on_input_players(
                changed,
                battle_session,
                menu.unwrap(),
                &mut self.th19,
            ),
            JunowenState::GameLoading { .. } => Ok(()),
            JunowenState::Game { battle_session } => {
                game::update_th19(battle_session, &mut self.th19)
            }
            JunowenState::BackToSelect { .. } => Ok(()),
        }
    }

    pub fn on_input_players(&mut self) {
        let Some((changed, menu)) = self.update_state() else {
            return;
        };
        if let Err(err) = self.update_th19_on_input_players(changed, menu) {
            debug!("session aborted: {}", err);
            self.end_session();
        }
    }

    pub fn on_input_menu(&mut self) {
        match &mut self.junowen_state {
            JunowenState::Standby => standby::on_input_menu(self),
            JunowenState::Prepare {
                state: prepare_state,
                ..
            } => prepare::update_th19_on_input_menu(&mut self.th19, *prepare_state),
            JunowenState::Select { battle_session } => {
                if let Err(err) = select::update_th19_on_input_menu(battle_session, &mut self.th19)
                {
                    debug!("session aborted: {}", err);
                    self.end_session();
                }
            }
            JunowenState::GameLoading { .. } => {}
            JunowenState::Game { .. } => {}
            JunowenState::BackToSelect { .. } => {}
        }
    }

    pub fn render_object(&self, old: Fn0b7d40, obj_renderer: *const c_void, obj: *const c_void) {
        if self.junowen_state.session().is_none() {
            standby::render_object(self, old, obj_renderer, obj);
            return;
        }
        old(obj_renderer, obj);
    }

    pub fn render_text(
        &mut self,
        old: Fn0d5ae0,
        text_renderer: *const c_void,
        text: &mut RenderingText,
    ) -> u32 {
        if self.junowen_state.session().is_none() {
            return standby::render_text(self, old, text_renderer, text);
        }
        old(text_renderer, text)
    }

    pub fn on_render_texts(&mut self, text_renderer: *const c_void) {
        let Some(session) = self.junowen_state.session() else {
            standby::on_render_texts(self, text_renderer);
            return;
        };
        in_session::on_render_texts(session, self, text_renderer);
    }

    pub fn on_round_over(&mut self) {
        self.junowen_state.on_round_over(&mut self.th19);
    }

    pub fn is_online_vs(&self, this: *const Selection, old: Fn011560) -> u8 {
        self.junowen_state.is_online_vs(this, old)
    }

    pub fn on_rewrite_controller_assignments(&mut self, old_fn: fn(&mut Th19) -> Fn10f720) {
        self.junowen_state
            .on_rewrite_controller_assignments(&mut self.th19, old_fn);
    }

    pub fn on_loaded_game_settings(&mut self) {
        self.junowen_state.on_loaded_game_settings(&mut self.th19);
    }
}
