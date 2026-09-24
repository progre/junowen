mod in_session;
mod spectator_game;
mod spectator_standby;

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
use tracing::warn;

use crate::{
    helper::pushed_escape,
    session::{
        RoundInitial,
        spectator::{GameInitial, SpectatorSession as SpectatorSessionProps},
    },
};

use super::prepare::Prepare;

use {spectator_game::SpectatorGame, spectator_standby::SpectatorStandby};

fn set_rand_seeds(th19: &mut Th19, round_initial: &RoundInitial) {
    th19.set_rand_seed1(round_initial.seed1).unwrap();
    th19.set_rand_seed2(round_initial.seed2).unwrap();
    th19.set_rand_seed3(round_initial.seed3).unwrap();
    th19.set_rand_seed4(round_initial.seed4).unwrap();
}

/// 観戦者の試合開始時の状態がホストと一致しているか確認する
fn verify_game_initial(th19: &Th19, init: &GameInitial) {
    let selection = th19.selection();
    let actual = (
        selection.difficulty as u8,
        selection.p1().character as u8,
        selection.p1().card as u8,
        selection.p2().character as u8,
        selection.p2().card as u8,
    );
    let expected = (
        init.difficulty(),
        init.p1().character(),
        init.p1().card(),
        init.p2().character(),
        init.p2().card(),
    );
    if actual != expected {
        warn!(
            "game initial mismatch. expected={:?}, actual={:?}",
            expected, actual
        );
    }
}

pub struct SpectatorSession {
    props: SpectatorSessionProps,
    state: SpectatorSessionState,
}

enum SpectatorSessionState {
    Prepare(Prepare),
    Standby(SpectatorStandby),
    /// 最初のフレームで乱数シードを設定する
    GameLoading(Option<RoundInitial>),
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

    pub fn change_to_standby(&mut self, first_time: bool) {
        self.state = SpectatorSessionState::Standby(SpectatorStandby::new(first_time));
    }
    pub fn change_to_game_loading(&mut self, round_initial: Option<RoundInitial>) {
        self.state = SpectatorSessionState::GameLoading(round_initial);
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
                    self.change_to_standby(true);
                }
                Some(Some(main_menu))
            }
            SpectatorSessionState::Standby(standby) => {
                // 待機中は観戦者側でキャンセルキーを入力することがあるため、キーボードの ESC で中断する
                if pushed_escape(th19.input_devices()) {
                    return None;
                }
                let main_menu = th19.app().main_loop_tasks().find_main_menu()?;
                match main_menu.screen_id() {
                    ScreenId::PlayerMatchupSelect => None,
                    ScreenId::GameLoading => {
                        let init = standby.take_game_initial();
                        if let Some(init) = &init {
                            verify_game_initial(th19, init);
                        }
                        self.change_to_game_loading(init.map(|x| x.round_initial().clone()));
                        Some(Some(main_menu))
                    }
                    _ => Some(Some(main_menu)),
                }
            }
            SpectatorSessionState::GameLoading(_) => {
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
            SpectatorSessionState::BackToSelect => {
                let Some(main_menu) = th19.app().main_loop_tasks().find_main_menu() else {
                    return Some(None);
                };
                if main_menu.screen_id() != ScreenId::CharacterSelect {
                    return Some(Some(main_menu));
                }
                self.change_to_standby(false);
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
            SpectatorSessionState::Standby(standby) => {
                standby.update_th19_on_input_players(&mut self.props, menu.unwrap(), th19)?
            }
            SpectatorSessionState::GameLoading(round_initial) => {
                if let Some(round_initial) = round_initial.take() {
                    set_rand_seeds(th19, &round_initial);
                }
                if th19.no_wait() {
                    th19.set_no_wait(false);
                }
            }
            SpectatorSessionState::Game(game) => game.update_th19(&mut self.props, th19)?,
            SpectatorSessionState::BackToSelect => {}
        }
        Ok(())
    }

    pub fn on_input_menu(&mut self, th19: &mut Th19) -> Result<bool, RecvError> {
        match &mut self.state {
            SpectatorSessionState::Prepare(prepare) => prepare.update_th19_on_input_menu(th19),
            SpectatorSessionState::Standby(standby) => {
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
                standby.update_th19_on_input_menu(&mut self.props, main_menu, th19)?;
            }
            SpectatorSessionState::GameLoading(_) => {}
            SpectatorSessionState::Game { .. } => {}
            SpectatorSessionState::BackToSelect => {}
        }
        Ok(true)
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: &c_void) {
        let waiting_for_host = matches!(
            &self.state,
            SpectatorSessionState::Standby(standby) if standby.is_waiting_for_host()
        );
        let Some(initial) = self.props.spectator_initial() else {
            if waiting_for_host {
                in_session::on_render_texts_waiting_for_host(th19, text_renderer);
            }
            return;
        };
        in_session::on_render_texts_spectator(
            th19,
            text_renderer,
            initial.p1_name(),
            initial.p2_name(),
            waiting_for_host,
        );
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let SpectatorSessionState::Game(game) = &mut self.state else {
            return Ok(());
        };
        game.on_round_over(&mut self.props, th19)
    }
}
