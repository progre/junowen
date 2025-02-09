mod session;
mod standby;

use std::ffi::c_void;

use junowen_lib::{Th19, structs::app::MainMenu, structs::others::RenderingText};
use session::Session;
use tracing::{debug, trace};

use crate::{
    file::Features,
    in_game_lobby::{Lobby, TitleMenuModifier},
};

pub enum JunowenState {
    Standby,
    Session(Session),
}

impl JunowenState {
    fn start_session(&mut self, session: Session) {
        *self = Self::Session(session);
    }

    fn end_session(&mut self) {
        *self = Self::Standby;
    }

    fn abort_session(&mut self, th19: &mut Th19) {
        self.end_session();
        th19.set_no_wait(false);
    }

    pub fn update_state(
        &mut self,
        th19: &Th19,
        lobby: &mut Lobby,
    ) -> (bool, Option<&'static MainMenu>) {
        match self {
            Self::Standby => {
                if let Some(session) = standby::update_state(th19, lobby.waiting_for_match_mut()) {
                    trace!("session received");
                    self.start_session(session);
                    lobby.clear_input();
                    return (true, None);
                }
                (false, None)
            }
            Self::Session(session) => {
                let Some(menu_opt) = session.update_state(th19) else {
                    self.end_session();
                    return (true, None);
                };
                (false, menu_opt)
            }
        }
    }

    pub fn update_th19_on_input_players(
        &mut self,
        changed: bool,
        menu: Option<&MainMenu>,
        th19: &mut Th19,
    ) {
        match self {
            Self::Standby => {
                if changed {
                    th19.set_no_wait(false);
                }
            }
            Self::Session(session) => {
                if let Err(err) = session.update_th19_on_input_players(menu, th19) {
                    debug!("session aborted: {err}");
                    self.abort_session(th19);
                }
            }
        }
    }

    pub fn on_input_menu(
        &mut self,
        th19: &mut Th19,
        title_menu_modifier: &mut TitleMenuModifier,
        lobby: &mut Lobby,
    ) {
        match self {
            Self::Standby => {
                standby::update_th19_on_input_menu(th19, title_menu_modifier, lobby);
            }
            Self::Session(session) => match session.on_input_menu(th19) {
                Ok(true) => {}
                Ok(false) => {
                    self.abort_session(th19);
                }
                Err(err) => {
                    debug!("session aborted: {err}");
                    self.abort_session(th19);
                }
            },
        }
    }

    pub fn on_before_render_object(
        &self,
        title_menu_modifier: &TitleMenuModifier,
        obj: &c_void,
    ) -> bool {
        match self {
            Self::Standby => standby::on_before_render_object(title_menu_modifier, obj),
            Self::Session(_) => true,
        }
    }

    pub fn on_before_render_text(
        &self,
        th19: &Th19,
        title_menu_modifier: &TitleMenuModifier,
        text_renderer: &c_void,
        text: &mut RenderingText,
    ) {
        match self {
            Self::Standby => {
                standby::on_before_render_text(th19, title_menu_modifier, text_renderer, text);
            }
            Self::Session(_) => {}
        }
    }

    pub fn on_render_texts(
        &self,
        features: &[Features],
        th19: &Th19,
        title_menu_modifier: &TitleMenuModifier,
        lobby: &Lobby,
        text_renderer: &c_void,
    ) {
        match self {
            Self::Standby => {
                standby::on_render_texts(th19, title_menu_modifier, lobby, text_renderer);
            }
            Self::Session(session) => {
                session.on_render_texts(features, th19, text_renderer);
            }
        }
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) {
        match self {
            Self::Standby => {}
            Self::Session(session) => {
                if let Err(err) = session.on_round_over(th19) {
                    debug!("session aborted: {err}");
                    self.abort_session(th19);
                }
            }
        }
    }

    pub fn on_before_is_online_vs(&self) -> Option<u8> {
        match self {
            Self::Standby => None,
            Self::Session(_) => Some(1),
        }
    }

    pub fn on_rewrite_controller_assignments(&mut self, th19: &mut Th19, old_p1_idx: u32) {
        match self {
            Self::Standby => {}
            Self::Session(_) => {
                if old_p1_idx != 0 {
                    return;
                }
                let input_devices = th19.input_devices_mut();
                if input_devices.p1_idx() == 0 {
                    return;
                }
                input_devices.set_p1_idx(0);
            }
        }
    }

    pub fn on_before_loaded_game_settings(&self, th19: &mut Th19) {
        match self {
            Self::Standby => {}
            Self::Session(session) => {
                let Some(game_settings) = session.game_settings() else {
                    return;
                };
                th19.put_game_settings_in_game(game_settings).unwrap();
            }
        }
    }
}
