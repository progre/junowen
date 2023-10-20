mod game;
mod in_session;
mod prepare;
mod select;
mod standby;

use std::{ffi::c_void, sync::mpsc::RecvError};

use anyhow::Result;
use getset::{Getters, MutGetters};
use junowen_lib::{Fn0b7d40, Fn0d5ae0, Fn10f720, Menu, RenderingText, ScreenId, Th19};
use tokio::sync::mpsc;
use tracing::{debug, trace};

use crate::{
    in_game_lobby::{Lobby, TitleMenuModifier},
    session::{MatchInitial, Session},
};

enum NetBattleState<'a> {
    Standby,
    Prepare {
        /// 0: back to title, 1: resolve controller, 2: forward to difficulty select
        state: u8,
    },
    Select {
        session: &'a mut Session,
        th19: &'a mut Th19,
        match_initial: &'a mut Option<MatchInitial>,
    },
    GameLoading,
    Game {
        session: &'a mut Session,
        th19: &'a mut Th19,
    },
    BackToSelect,
}

#[derive(Getters, MutGetters)]
pub struct State {
    session_rx: mpsc::Receiver<Session>,
    #[getset(get = "pub", get_mut = "pub")]
    th19: Th19,
    state: u8,
    #[getset(get = "pub")]
    session: Option<Session>,
    match_initial: Option<MatchInitial>,
    title_menu_modifier: TitleMenuModifier,
    lobby: Lobby,
}

impl State {
    pub fn new(th19: Th19) -> Self {
        let (session_tx, session_rx) = mpsc::channel(1);
        Self {
            session_rx,
            th19,
            state: 0x00,
            session: None,
            match_initial: None,
            title_menu_modifier: TitleMenuModifier::new(),
            lobby: Lobby::new(session_tx),
        }
    }

    fn net_battle_state(&mut self) -> NetBattleState {
        match self.state {
            0x00 => NetBattleState::Standby,
            0x10..=0x12 => NetBattleState::Prepare {
                state: self.state - 0x10,
            },
            0x20 | 0x21 => NetBattleState::Select {
                session: self.session.as_mut().unwrap(),
                th19: &mut self.th19,
                match_initial: &mut self.match_initial,
            },
            0x30 => NetBattleState::GameLoading,
            0x40 => NetBattleState::Game {
                session: self.session.as_mut().unwrap(),
                th19: &mut self.th19,
            },
            0x50 => NetBattleState::BackToSelect,
            _ => unreachable!(),
        }
    }

    fn start_session(&mut self, session: Session) {
        self.state = 0x10;
        self.session = Some(session);
    }

    fn change_to_prepare(&mut self, state: u8) {
        self.state = 0x10 + state;
    }

    fn change_to_select(&mut self) {
        self.state = 0x20;
    }
    fn change_to_game_loading(&mut self) {
        self.state = 0x30;
    }
    fn change_to_game(&mut self) {
        self.state = 0x40;
    }
    fn change_to_back_to_select(&mut self) {
        self.state = 0x50;
    }

    fn end_session(&mut self) {
        self.state = 0x00;
        self.session = None;
        self.match_initial = None;
        self.lobby.reset_depth();
    }
}

fn update_state(state: &mut State) -> Option<(bool, Option<&'static Menu>)> {
    match state.net_battle_state() {
        NetBattleState::Standby => {
            if let Ok(session) = state.session_rx.try_recv() {
                trace!("session received");
                state.start_session(session);
                return Some((true, None));
            };
            None
        }
        NetBattleState::Prepare {
            state: prepare_state,
        } => prepare::update_state(state, prepare_state),
        NetBattleState::Select { .. } => select::update_state(state),
        NetBattleState::GameLoading => {
            let Some(game) = state.th19.round() else {
                return Some((false, None));
            };
            if !game.is_first_frame() {
                return Some((false, None));
            }
            state.change_to_game();
            Some((true, None))
        }
        NetBattleState::Game { .. } => game::update_state(state),
        NetBattleState::BackToSelect => {
            let Some(menu) = state.th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
                return Some((false, None));
            };
            if menu.screen_id != ScreenId::CharacterSelect {
                return Some((false, Some(menu)));
            }
            state.change_to_select();
            Some((true, Some(menu)))
        }
    }
}

fn update_th19_on_input_players(
    state: &mut State,
    changed: bool,
    menu: Option<&Menu>,
) -> Result<(), RecvError> {
    match state.net_battle_state() {
        NetBattleState::Standby => unreachable!(),
        NetBattleState::Prepare {
            state: prepare_state,
        } => {
            prepare::update_th19_on_input_players(&mut state.th19, prepare_state);
            Ok(())
        }
        NetBattleState::Select {
            th19,
            session,
            match_initial,
        } => select::update_th19_on_input_players(
            changed,
            session,
            menu.unwrap(),
            th19,
            match_initial,
        ),
        NetBattleState::GameLoading => Ok(()),
        NetBattleState::Game { session, th19 } => game::update_th19(session, th19),
        NetBattleState::BackToSelect => Ok(()),
    }
}

pub fn on_input_players(state: &mut State) {
    let Some((changed, menu)) = update_state(state) else {
        return;
    };
    if let Err(err) = update_th19_on_input_players(state, changed, menu) {
        debug!("session aborted: {}", err);
        state.end_session();
    }
}

pub fn on_input_menu(state: &mut State) {
    match state.net_battle_state() {
        NetBattleState::Standby => standby::on_input_menu(state),
        NetBattleState::Prepare {
            state: prepare_state,
        } => prepare::update_th19_on_input_menu(&mut state.th19, prepare_state),
        NetBattleState::Select {
            th19,
            session,
            match_initial: _,
        } => {
            if let Err(err) = select::update_th19_on_input_menu(session, th19) {
                debug!("session aborted: {}", err);
                state.end_session();
            }
        }
        NetBattleState::GameLoading => {}
        NetBattleState::Game { .. } => {}
        NetBattleState::BackToSelect => {}
    }
}

pub fn render_object(
    state: &State,
    old: Fn0b7d40,
    obj_renderer: *const c_void,
    obj: *const c_void,
) {
    if state.session().is_none() {
        standby::render_object(state, old, obj_renderer, obj);
        return;
    }
    old(obj_renderer, obj);
}

pub fn render_text(
    state: &mut State,
    old: Fn0d5ae0,
    text_renderer: *const c_void,
    text: &mut RenderingText,
) -> u32 {
    if state.session().is_none() {
        return standby::render_text(state, old, text_renderer, text);
    }
    old(text_renderer, text)
}

pub fn on_render_texts(state: &mut State, text_renderer: *const c_void) {
    let Some(session) = state.session() else {
        standby::on_render_texts(state, text_renderer);
        return;
    };
    in_session::on_render_texts(session, state, text_renderer);
}

pub fn on_round_over(state: &mut State) {
    let Some(session) = &mut state.session else {
        return;
    };
    if let Err(err) = game::on_round_over(session, &mut state.th19) {
        debug!("session aborted: {}", err);
        state.end_session();
    }
}

pub fn on_rewrite_controller_assignments(state: &mut State, old_fn: fn(&mut Th19) -> Fn10f720) {
    if state.session.is_none() {
        old_fn(state.th19_mut())();
        return;
    }
    in_session::on_rewrite_controller_assignments(state.th19_mut(), old_fn);
}

pub fn on_loaded_game_settings(state: &mut State) {
    if let Some(match_initial) = &state.match_initial {
        select::on_loaded_game_settings(match_initial, &mut state.th19);
    }
}
