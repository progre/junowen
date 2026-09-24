use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use junowen_lib::{
    Th19,
    structs::{
        app::{MainMenu, ScreenId},
        input_devices::InputValue,
    },
    th19_helpers::{reset_cursors, shot_repeatedly},
};
use tracing::trace;

use crate::{
    helper::pushed_f1,
    session::{
        RoundInitial,
        spectator::{self, SpectatorSession},
    },
    state::battle_session::spectator_host::{SpectatorHostState, SpectatorMatchInfo},
};

fn set_rand_seeds(th19: &mut Th19, round_initial: &RoundInitial) {
    th19.set_rand_seed1(round_initial.seed1).unwrap();
    th19.set_rand_seed2(round_initial.seed2).unwrap();
    th19.set_rand_seed3(round_initial.seed3).unwrap();
    th19.set_rand_seed4(round_initial.seed4).unwrap();
}

enum InitializingState {
    Uninitialized,
    /// ホストからの初期化情報 (同期ポイントの状態) を待っている
    WaitingForHost,
    /// 難易度選択画面の同期ポイントの状態を再現している
    SyncingToDifficultySelect,
    /// キャラクター選択画面の同期ポイントへ移動している
    ///
    /// ホストの同期ポイントはキャラクター選択画面に入った時点なので、
    /// 難易度の決定や画面遷移を経た後、画面に到達した時点で乱数シードを設定する
    SyncingToCharacterSelect {
        round_initial: RoundInitial,
    },
    Synced,
}

#[derive(new)]
pub struct SpectatorSelect {
    #[new(value = "InitializingState::Uninitialized")]
    initializing_state: InitializingState,
}

impl SpectatorSelect {
    pub fn is_waiting_for_host(&self) -> bool {
        matches!(
            self.initializing_state,
            InitializingState::Uninitialized | InitializingState::WaitingForHost
        )
    }

    pub fn update_th19_on_input_players(
        &mut self,
        session: &mut SpectatorSession,
        mut relay: Option<&mut SpectatorHostState>,
        main_menu: &MainMenu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if let InitializingState::Uninitialized = self.initializing_state {
            if session.spectator_initial().is_none() {
                self.initializing_state = InitializingState::WaitingForHost;
                reset_cursors(th19);
            } else {
                // 対戦後にキャラクター選択画面へ戻ってきた
                let round_initial = session.dequeue_init_round()?;
                set_rand_seeds(th19, &round_initial);
                if let Some(relay) = relay.as_deref_mut() {
                    relay.send_init_round_if_connected(th19);
                }
                self.initializing_state = InitializingState::Synced;
            }
        }
        if let InitializingState::WaitingForHost = self.initializing_state {
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
            let screen = session
                .spectator_initial()
                .unwrap()
                .initial_state()
                .screen();
            self.initializing_state = match screen {
                spectator::Screen::DifficultySelect => {
                    // 既に難易度選択画面にいるので、そのまま設定する
                    set_rand_seeds(th19, &round_initial);
                    InitializingState::SyncingToDifficultySelect
                }
                spectator::Screen::CharacterSelect => {
                    InitializingState::SyncingToCharacterSelect { round_initial }
                }
                spectator::Screen::Game => unimplemented!(),
            };
        }
        if main_menu.screen_id() == ScreenId::DifficultySelect {
            return Ok(());
        }
        match &self.initializing_state {
            InitializingState::Uninitialized | InitializingState::WaitingForHost => unreachable!(),
            InitializingState::SyncingToDifficultySelect => return Ok(()),
            InitializingState::SyncingToCharacterSelect { round_initial } => {
                if main_menu.screen_id() != ScreenId::CharacterSelect {
                    return Ok(());
                }
                // キャラクター選択画面に到達したので、ホストが同期ポイントに入った時点の状態を再現する
                let initial_state = session.spectator_initial().unwrap().initial_state();
                let selection = th19.selection_mut();
                selection.p1_mut().card = initial_state.p1_card() as u32;
                selection.p2_mut().card = initial_state.p2_card() as u32;
                set_rand_seeds(th19, round_initial);
                self.initializing_state = InitializingState::Synced;
            }
            InitializingState::Synced => {}
        }
        if !th19.no_wait() {
            th19.set_no_wait(true);
        }

        let (p1, p2) = session.dequeue_inputs()?;
        let f1_pushed = pushed_f1(th19.input_devices());
        let input_devices = th19.input_devices_mut();
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());

        if let Some(relay) = relay {
            let match_info =
                SpectatorMatchInfo::from_spectator_initial(session.spectator_initial().unwrap());
            relay.update(f1_pushed, Some(main_menu), th19, match_info, p1, p2);
        }

        Ok(())
    }

    pub fn update_th19_on_input_menu(
        &mut self,
        session: &mut SpectatorSession,
        relay: Option<&mut SpectatorHostState>,
        main_menu: &mut MainMenu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if main_menu.screen_id() != ScreenId::DifficultySelect {
            return Ok(());
        }
        match &self.initializing_state {
            InitializingState::Uninitialized | InitializingState::WaitingForHost => {
                // 同期前はホストの入力を消費せず、観戦者の操作も受け付けない
                th19.menu_input_mut().set_current(InputValue::empty());
                return Ok(());
            }
            InitializingState::SyncingToDifficultySelect
            | InitializingState::SyncingToCharacterSelect { .. } => {
                let init = session.spectator_initial().unwrap();
                trace!("spectator_initial: {:?}", init);
                let initial_state = init.initial_state();
                let menu = main_menu.menu_mut();
                if menu.cursor() != initial_state.difficulty() as u32 {
                    menu.set_cursor(initial_state.difficulty() as u32);
                    th19.menu_input_mut().set_current(InputValue::empty());
                    return Ok(());
                }
                let selection = th19.selection_mut();
                selection.p1_mut().character = initial_state.p1_character() as u32;
                selection.p2_mut().character = initial_state.p2_character() as u32;
                if let InitializingState::SyncingToCharacterSelect { .. } = self.initializing_state
                {
                    // 難易度を決定してキャラクター選択画面へ進む
                    let prev = th19.menu_input().prev();
                    th19.menu_input_mut().set_current(shot_repeatedly(prev));
                    return Ok(());
                }
                th19.set_no_wait(false);
                self.initializing_state = InitializingState::Synced;
            }
            InitializingState::Synced => {}
        }

        let (p1, p2) = session.dequeue_inputs()?;
        let input = if p1 != 0 { p1 } else { p2 };
        th19.menu_input_mut()
            .set_current((input as u32).try_into().unwrap());

        if let Some(relay) = relay {
            let f1_pushed = pushed_f1(th19.input_devices());
            let match_info =
                SpectatorMatchInfo::from_spectator_initial(session.spectator_initial().unwrap());
            relay.update(f1_pushed, Some(main_menu), th19, match_info, p1, p2);
        }
        Ok(())
    }
}
