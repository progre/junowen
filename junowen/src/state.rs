mod battle_session_state;
mod junowen_state;
mod prepare;
mod render_parts;
mod spectator_session_state;

use std::{ffi::c_void, fmt::Display};

use getset::{Getters, MutGetters};
use junowen_lib::{structs::others::RenderingText, Th19};
use tracing::{debug, trace};

use self::junowen_state::JunowenState;
use crate::{
    file::{Features, SettingsRepo},
    in_game_lobby::{Lobby, TitleMenuModifier},
};

#[derive(Getters, MutGetters)]
pub struct State {
    features: Vec<Features>,
    #[getset(get_mut = "pub")]
    th19: Th19,
    title_menu_modifier: TitleMenuModifier,
    lobby: Lobby,
    junowen_state: JunowenState,
    old_p1_idx: u32,
}

impl State {
    pub async fn new(settings_repo: SettingsRepo, th19: Th19) -> Self {
        Self {
            features: settings_repo.features().await,
            th19,
            title_menu_modifier: TitleMenuModifier::new(),
            lobby: Lobby::new(settings_repo),
            junowen_state: JunowenState::Standby,
            old_p1_idx: 0,
        }
    }

    fn abort_session(&mut self, err: impl Display) {
        debug!("session aborted: {}", err);
        self.junowen_state.abort_session(&mut self.th19);
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

    pub fn on_before_render_object(&self, obj: *const c_void) -> bool {
        self.junowen_state
            .on_before_render_object(&self.title_menu_modifier, obj)
    }

    pub fn on_before_render_text(&self, text_renderer: *const c_void, text: &mut RenderingText) {
        self.junowen_state.on_before_render_text(
            &self.th19,
            &self.title_menu_modifier,
            text_renderer,
            text,
        );
    }

    pub fn on_render_texts(&self, text_renderer: *const c_void) {
        self.junowen_state.on_render_texts(
            &self.features,
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

    pub fn on_before_is_online_vs(&self) -> Option<u8> {
        self.junowen_state.on_before_is_online_vs()
    }

    pub fn on_before_rewrite_controller_assignments(&mut self) {
        let input_devices = self.th19.input_devices();
        self.old_p1_idx = input_devices.p1_idx();
    }

    pub fn on_rewrite_controller_assignments(&mut self) {
        if !self.junowen_state.has_session() {
            return;
        }
        trace!(
            "on_rewrite_controller_assignments: before old_p1_idx={}",
            self.old_p1_idx
        );
        if self.old_p1_idx != 0 {
            return;
        }
        let input_devices = self.th19.input_devices_mut();
        if input_devices.p1_idx() == 0 {
            return;
        }
        trace!(
            "on_rewrite_controller_assignments: after input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
        input_devices.set_p1_idx(0);
        trace!(
            "on_rewrite_controller_assignments: fixed input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
    }

    pub fn on_before_loaded_game_settings(&mut self) {
        self.junowen_state
            .on_before_loaded_game_settings(&mut self.th19);
    }
}
