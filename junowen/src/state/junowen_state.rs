use std::mem;

use junowen_lib::{Fn011560, Fn10f720, Selection, Th19};
use tracing::debug;

use crate::session::BattleSession;

use super::{game, in_session};

pub enum JunowenState {
    Standby,
    Prepare {
        battle_session: BattleSession,
        /// 0: back to title, 1: resolve controller, 2: forward to difficulty select
        state: u8,
    },
    Select {
        battle_session: BattleSession,
    },
    GameLoading {
        battle_session: BattleSession,
    },
    Game {
        battle_session: BattleSession,
    },
    BackToSelect {
        battle_session: BattleSession,
    },
}

impl JunowenState {
    pub fn session(&self) -> Option<&BattleSession> {
        match self {
            JunowenState::Standby => None,
            JunowenState::Prepare { battle_session, .. }
            | JunowenState::Select { battle_session }
            | JunowenState::GameLoading { battle_session }
            | JunowenState::Game { battle_session }
            | JunowenState::BackToSelect { battle_session } => Some(battle_session),
        }
    }

    pub fn session_mut(&mut self) -> Option<&mut BattleSession> {
        match self {
            JunowenState::Standby => None,
            JunowenState::Prepare { battle_session, .. }
            | JunowenState::Select { battle_session }
            | JunowenState::GameLoading { battle_session }
            | JunowenState::Game { battle_session }
            | JunowenState::BackToSelect { battle_session } => Some(battle_session),
        }
    }

    pub fn inner_session(self) -> BattleSession {
        match self {
            JunowenState::Standby => unreachable!(),
            JunowenState::Prepare { battle_session, .. }
            | JunowenState::Select { battle_session }
            | JunowenState::GameLoading { battle_session }
            | JunowenState::Game { battle_session }
            | JunowenState::BackToSelect { battle_session } => battle_session,
        }
    }

    pub fn start_session(&mut self, battle_session: BattleSession) {
        *self = JunowenState::Prepare {
            battle_session,
            state: 0,
        };
    }

    pub fn change_to_prepare(&mut self, new_state: u8) {
        let JunowenState::Prepare { state, .. } = self else {
            unreachable!();
        };
        *state = new_state;
    }

    pub fn change_to_select(&mut self) {
        let old = mem::replace(self, JunowenState::Standby);
        *self = JunowenState::Select {
            battle_session: old.inner_session(),
        };
    }
    pub fn change_to_game_loading(&mut self) {
        let old = mem::replace(self, JunowenState::Standby);
        *self = JunowenState::GameLoading {
            battle_session: old.inner_session(),
        }
    }
    pub fn change_to_game(&mut self) {
        let old = mem::replace(self, JunowenState::Standby);
        *self = JunowenState::Game {
            battle_session: old.inner_session(),
        }
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

    pub fn on_round_over(&mut self, th19: &mut Th19) {
        let Some(session) = self.session_mut() else {
            return;
        };
        if let Err(err) = game::on_round_over(session, th19) {
            debug!("session aborted: {}", err);
            self.end_session();
        }
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
