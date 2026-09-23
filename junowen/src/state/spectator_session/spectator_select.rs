use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use getset::{Getters, MutGetters};
use junowen_lib::{
    Th19,
    structs::{
        app::{MainMenu, ScreenId},
        input_devices::InputValue,
    },
    th19_helpers::{reset_cursors, shot_repeatedly},
};
use tracing::trace;

use crate::session::{
    RoundInitial,
    spectator::{self, SpectatorSession},
};

fn set_rand_seeds(th19: &mut Th19, round_initial: &RoundInitial) {
    th19.set_rand_seed1(round_initial.seed1).unwrap();
    th19.set_rand_seed2(round_initial.seed2).unwrap();
    th19.set_rand_seed3(round_initial.seed3).unwrap();
    th19.set_rand_seed4(round_initial.seed4).unwrap();
}

#[derive(new, Getters, MutGetters)]
pub struct SpectatorSelect {
    /// 0: 未初期化, 1: ホストからの初期化情報待ち, 2: 同期ポイントへ移動中, 3: 同期済み
    #[new(value = "0")]
    initializing_state: u8,
    /// キャラクター選択画面から同期する場合に、画面到達時に設定する乱数シード
    #[new(default)]
    round_initial: Option<RoundInitial>,
}

impl SpectatorSelect {
    pub fn is_waiting_for_host(&self) -> bool {
        self.initializing_state <= 1
    }

    pub fn update_th19_on_input_players(
        &mut self,
        session: &mut SpectatorSession,
        main_menu: &MainMenu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if self.initializing_state == 0 {
            if session.spectator_initial().is_none() {
                self.initializing_state = 1;
                reset_cursors(th19);
            } else {
                self.initializing_state = 3;
                let round_initial = session.dequeue_init_round()?;
                set_rand_seeds(th19, &round_initial);
            }
        }
        if self.initializing_state == 1 {
            if !session.try_recv_init_spectator()? {
                // ホストの準備が整うまで待機する
                if th19.no_wait() {
                    th19.set_no_wait(false);
                }
                let input_devices = th19.input_devices_mut();
                input_devices
                    .p1_input_mut()
                    .set_current(InputValue::empty());
                input_devices
                    .p2_input_mut()
                    .set_current(InputValue::empty());
                return Ok(());
            }
            let round_initial = session.dequeue_init_round()?;
            set_rand_seeds(th19, &round_initial);
            self.round_initial = Some(round_initial);
            self.initializing_state = 2;
        }
        if main_menu.screen_id() == ScreenId::DifficultySelect {
            return Ok(());
        }
        if self.initializing_state == 2 {
            let initial_state = session.spectator_initial().unwrap().initial_state().clone();
            match initial_state.screen() {
                spectator::Screen::DifficultySelect => {
                    return Ok(());
                }
                spectator::Screen::CharacterSelect => {
                    if main_menu.screen_id() != ScreenId::CharacterSelect {
                        return Ok(());
                    }
                    // キャラクター選択画面に到達したので、ホストが同期ポイントに入った時点の状態を再現する
                    let selection = th19.selection_mut();
                    selection.p1_mut().card = initial_state.p1_card() as u32;
                    selection.p2_mut().card = initial_state.p2_card() as u32;
                    let round_initial = self.round_initial.take().unwrap();
                    set_rand_seeds(th19, &round_initial);
                    self.initializing_state = 3;
                }
                spectator::Screen::Game => unimplemented!(),
            }
        }
        if !th19.no_wait() {
            th19.set_no_wait(true);
        }

        let (p1, p2) = session.dequeue_inputs()?;
        let input_devices = th19.input_devices_mut();
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());

        Ok(())
    }

    pub fn update_th19_on_input_menu(
        &mut self,
        session: &mut SpectatorSession,
        main_menu: &mut MainMenu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if main_menu.screen_id() != ScreenId::DifficultySelect {
            return Ok(());
        }
        if self.is_waiting_for_host() {
            th19.menu_input_mut().set_current(InputValue::empty());
            return Ok(());
        }
        let menu = main_menu.menu_mut();
        if self.initializing_state == 2 {
            let init = session.spectator_initial().unwrap();
            trace!("spectator_initial: {:?}", init);
            let initial_state = init.initial_state().clone();
            if menu.cursor() != initial_state.difficulty() as u32 {
                menu.set_cursor(initial_state.difficulty() as u32);
                th19.menu_input_mut().set_current(InputValue::empty());
                return Ok(());
            }
            let selection = th19.selection_mut();
            selection.p1_mut().character = initial_state.p1_character() as u32;
            selection.p2_mut().character = initial_state.p2_character() as u32;
            match initial_state.screen() {
                spectator::Screen::DifficultySelect => {
                    th19.set_no_wait(false);
                    self.round_initial = None;
                    self.initializing_state = 3;
                }
                spectator::Screen::CharacterSelect => {
                    // 難易度を決定してキャラクター選択画面へ進む
                    let prev = th19.menu_input().prev();
                    th19.menu_input_mut().set_current(shot_repeatedly(prev));
                    return Ok(());
                }
                spectator::Screen::Game => unimplemented!(),
            }
        }

        let (p1, p2) = session.dequeue_inputs()?;
        let input = if p1 != 0 { p1 } else { p2 };
        th19.menu_input_mut()
            .set_current((input as u32).try_into().unwrap());
        Ok(())
    }
}
