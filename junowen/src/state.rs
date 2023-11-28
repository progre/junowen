mod battle_game;
mod battle_select;
mod battle_session_state;
mod in_session;
mod junowen_state;
mod prepare;
mod spectator_game;
mod spectator_host;
mod spectator_select;
mod spectator_session_state;
mod standby;

use std::{ffi::c_void, fmt::Display};

use getset::{Getters, MutGetters};
use junowen_lib::{Fn011560, Fn0b7d40, Fn0d5ae0, Fn10f720, RenderingText, Selection, Th19};
use tracing::debug;

use self::junowen_state::JunowenState;
use crate::in_game_lobby::{Lobby, TitleMenuModifier};

#[derive(Getters, MutGetters)]
pub struct State {
    #[getset(get_mut = "pub")]
    th19: Th19,
    title_menu_modifier: TitleMenuModifier,
    lobby: Lobby,
    junowen_state: JunowenState,
}

impl State {
    pub fn new(th19: Th19) -> Self {
        Self {
            th19,
            title_menu_modifier: TitleMenuModifier::new(),
            lobby: Lobby::new(),
            junowen_state: JunowenState::Standby,
        }
    }

    fn abort_session(&mut self, err: impl Display) {
        debug!("session aborted: {}", err);
        self.th19.set_no_wait(false);
        self.junowen_state.end_session();
        self.lobby.reset_depth();
    }

    pub fn on_input_players(&mut self) {
        let has_session = self.junowen_state.has_session();
        match self
            .junowen_state
            .on_input_players(&mut self.th19, self.lobby.waiting_for_match_mut())
        {
            Ok(_) => {
                if has_session && self.junowen_state.has_session() {
                    self.lobby.reset_depth();
                }
            }
            Err(err) => {
                self.abort_session(err);
            }
        }
    }

    pub fn on_input_menu(&mut self) {
        if let Err(err) = self.junowen_state.on_input_menu(
            &mut self.th19,
            &mut self.title_menu_modifier,
            &mut self.lobby,
        ) {
            self.abort_session(err);
        }
    }

    pub fn render_object(&self, old: Fn0b7d40, obj_renderer: *const c_void, obj: *const c_void) {
        self.junowen_state
            .render_object(&self.title_menu_modifier, old, obj_renderer, obj);
    }

    pub fn render_text(
        &self,
        old: Fn0d5ae0,
        text_renderer: *const c_void,
        text: &mut RenderingText,
    ) -> u32 {
        self.junowen_state.render_text(
            &self.th19,
            &self.title_menu_modifier,
            old,
            text_renderer,
            text,
        )
    }

    pub fn on_render_texts(&self, text_renderer: *const c_void) {
        self.junowen_state.on_render_texts(
            &self.th19,
            &self.title_menu_modifier,
            &self.lobby,
            text_renderer,
        );
    }

    pub fn on_round_over(&mut self) {
        if let Err(err) = self.junowen_state.on_round_over(&mut self.th19) {
            self.abort_session(err);
        }
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
