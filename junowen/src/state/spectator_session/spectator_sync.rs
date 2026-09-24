use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::{
    Th19,
    structs::{
        app::{MainMenu, ScreenId},
        input_devices::{InputFlags, InputValue},
    },
    th19_helpers::{reset_cursors, shot_repeatedly},
};
use tracing::trace;

use crate::session::{
    RoundInitial,
    spectator::{self, CharacterSelectPhase, SpectatorSession, has_card_select_phase},
};

use super::set_rand_seeds;

/// 進行段階を再現するときに、決定キーを押す間隔 (フレーム)
///
/// 画面の演出中に押すと無視される可能性があるため、間隔を空ける
const PRESS_INTERVAL_FRAMES: u32 = 20;

/// キャラクター選択画面で、ホストの各プレイヤーの進行段階を決定キーの押下で再現する
struct PhaseReproduction {
    phases: [CharacterSelectPhase; 2],
    wait: u32,
}

enum SpectatorSyncState {
    /// ホストからの初期化情報 (同期ポイントの状態) を待っている
    WaitingForHost,
    /// 難易度選択画面の同期ポイントの状態を再現している
    ToDifficultySelect,
    /// キャラクター選択画面へ移動し、ホストの状態を再現している
    ///
    /// ホストの状態はキャラクター選択画面の途中のものなので、難易度の決定や画面遷移、
    /// 進行段階の再現を経た後に乱数シードを設定する
    ToCharacterSelect {
        round_initial: RoundInitial,
        reproduction: Option<PhaseReproduction>,
    },
}

/// 観戦開始時に、ホストの同期ポイントと同じ状態まで移動する
///
/// 同期が完了したら、以降は `SpectatorSelect` がホストの入力を再生する
pub struct SpectatorSync {
    cursors_reset: bool,
    state: SpectatorSyncState,
}

impl SpectatorSync {
    pub fn new() -> Self {
        Self {
            cursors_reset: false,
            state: SpectatorSyncState::WaitingForHost,
        }
    }

    pub fn is_waiting_for_host(&self) -> bool {
        matches!(self.state, SpectatorSyncState::WaitingForHost)
    }

    /// 同期が完了したら `true` を返す。完了したフレームからホストの入力を再生する必要がある
    pub fn update_th19_on_input_players(
        &mut self,
        session: &mut SpectatorSession,
        main_menu: &MainMenu,
        th19: &mut Th19,
    ) -> Result<bool, RecvError> {
        if !self.cursors_reset {
            self.cursors_reset = true;
            reset_cursors(th19);
        }
        if let SpectatorSyncState::WaitingForHost = self.state {
            if !session.try_recv_init_spectator()? {
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
                return Ok(false);
            }
            let round_initial = session.dequeue_init_round()?;
            let screen = session
                .spectator_initial()
                .unwrap()
                .initial_state()
                .screen();
            self.state = match screen {
                spectator::Screen::DifficultySelect => {
                    // 既に難易度選択画面にいるので、そのまま設定する
                    set_rand_seeds(th19, &round_initial);
                    SpectatorSyncState::ToDifficultySelect
                }
                spectator::Screen::CharacterSelect => SpectatorSyncState::ToCharacterSelect {
                    round_initial,
                    reproduction: None,
                },
            };
        }
        let SpectatorSyncState::ToCharacterSelect {
            round_initial,
            reproduction,
        } = &mut self.state
        else {
            return Ok(false);
        };
        if main_menu.screen_id() != ScreenId::CharacterSelect {
            return Ok(false);
        }
        let spectator_initial = session.spectator_initial().unwrap();
        let has_card_phase = has_card_select_phase(spectator_initial.game_settings());
        let initial_state = spectator_initial.initial_state().clone();
        let set_cards = |th19: &mut Th19| {
            let selection = th19.selection_mut();
            selection.p1_mut().card = initial_state.p1().card() as u32;
            selection.p2_mut().card = initial_state.p2().card() as u32;
        };
        let reproduction = reproduction.get_or_insert_with(|| {
            // キャラクター選択画面に到達したので、カーソル位置を合わせる
            let menu = th19
                .app_mut()
                .main_loop_tasks_mut()
                .find_main_menu_mut()
                .unwrap()
                .menu_mut();
            let p1_cursor = menu.p1_cursor_mut();
            p1_cursor.cursor = initial_state.p1().character() as u32;
            p1_cursor.prev_cursor = p1_cursor.cursor;
            let p2_cursor = menu.p2_cursor_mut();
            p2_cursor.cursor = initial_state.p2().character() as u32;
            p2_cursor.prev_cursor = p2_cursor.cursor;
            set_cards(th19);
            PhaseReproduction {
                phases: [CharacterSelectPhase::Character; 2],
                wait: PRESS_INTERVAL_FRAMES,
            }
        });
        let targets = [initial_state.p1().phase(), initial_state.p2().phase()];
        if reproduction.phases == targets {
            // 段階の遷移でカードのカーソルが変わる可能性があるため、最後に改めて合わせる
            set_cards(th19);
            set_rand_seeds(th19, round_initial);
            return Ok(true);
        }

        if !th19.no_wait() {
            th19.set_no_wait(true);
        }
        let mut inputs = [InputValue::empty(); 2];
        if reproduction.wait > 0 {
            reproduction.wait -= 1;
        } else {
            for ((phase, target), input) in
                reproduction.phases.iter_mut().zip(targets).zip(&mut inputs)
            {
                if *phase == target {
                    continue;
                }
                if *phase == CharacterSelectPhase::Card {
                    // カードを選んでから決定する
                    set_cards(th19);
                }
                *input = InputFlags::SHOT.into();
                *phase = phase.decided(has_card_phase);
            }
            reproduction.wait = PRESS_INTERVAL_FRAMES;
        }
        let input_devices = th19.input_devices_mut();
        input_devices.p1_input_mut().set_current(inputs[0]);
        input_devices.p2_input_mut().set_current(inputs[1]);
        Ok(false)
    }

    /// 同期が完了したら `true` を返す。完了したフレームからホストの入力を再生する必要がある
    pub fn update_th19_on_input_menu(
        &mut self,
        session: &mut SpectatorSession,
        main_menu: &mut MainMenu,
        th19: &mut Th19,
    ) -> Result<bool, RecvError> {
        if main_menu.screen_id() != ScreenId::DifficultySelect {
            return Ok(false);
        }
        if let SpectatorSyncState::WaitingForHost = self.state {
            th19.menu_input_mut().set_current(InputValue::empty());
            return Ok(false);
        }
        let init = session.spectator_initial().unwrap();
        trace!("spectator_initial: {:?}", init);
        let initial_state = init.initial_state();
        let menu = main_menu.menu_mut();
        if menu.cursor() != initial_state.difficulty() as u32 {
            menu.set_cursor(initial_state.difficulty() as u32);
            th19.menu_input_mut().set_current(InputValue::empty());
            return Ok(false);
        }
        let selection = th19.selection_mut();
        selection.p1_mut().character = initial_state.p1().character() as u32;
        selection.p2_mut().character = initial_state.p2().character() as u32;
        match self.state {
            SpectatorSyncState::WaitingForHost => unreachable!(),
            SpectatorSyncState::ToDifficultySelect => {
                th19.set_no_wait(false);
                Ok(true)
            }
            SpectatorSyncState::ToCharacterSelect { .. } => {
                // 難易度を決定してキャラクター選択画面へ進む
                let prev = th19.menu_input().prev();
                th19.menu_input_mut().set_current(shot_repeatedly(prev));
                Ok(false)
            }
        }
    }
}
