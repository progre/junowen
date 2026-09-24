use std::sync::mpsc::RecvError;

use anyhow::Result;
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

use super::set_rand_seeds;

enum SpectatorSyncState {
    /// ホストからの初期化情報 (同期ポイントの状態) を待っている
    WaitingForHost,
    /// 難易度選択画面の同期ポイントの状態を再現している
    ToDifficultySelect,
    /// キャラクター選択画面の同期ポイントへ移動している
    ///
    /// ホストの同期ポイントはキャラクター選択画面に入った時点なので、
    /// 難易度の決定や画面遷移を経た後、画面に到達した時点で乱数シードを設定する
    ToCharacterSelect { round_initial: RoundInitial },
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
                spectator::Screen::CharacterSelect => {
                    SpectatorSyncState::ToCharacterSelect { round_initial }
                }
            };
        }
        let SpectatorSyncState::ToCharacterSelect { round_initial } = &self.state else {
            return Ok(false);
        };
        if main_menu.screen_id() != ScreenId::CharacterSelect {
            return Ok(false);
        }
        // キャラクター選択画面に到達したので、ホストが同期ポイントに入った時点の状態を再現する
        let initial_state = session.spectator_initial().unwrap().initial_state();
        let selection = th19.selection_mut();
        selection.p1_mut().card = initial_state.p1_card() as u32;
        selection.p2_mut().card = initial_state.p2_card() as u32;
        set_rand_seeds(th19, round_initial);
        Ok(true)
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
        selection.p1_mut().character = initial_state.p1_character() as u32;
        selection.p2_mut().character = initial_state.p2_character() as u32;
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
