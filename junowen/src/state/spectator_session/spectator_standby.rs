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
use tracing::{trace, warn};

use crate::session::spectator::{GameInitial, SpectatorSession};

fn clear_player_inputs(th19: &mut Th19) {
    let input_devices = th19.input_devices_mut();
    input_devices
        .p1_input_mut()
        .set_current(InputValue::empty());
    input_devices
        .p2_input_mut()
        .set_current(InputValue::empty());
}

/// キャラクター選択画面のカーソルとカードが試合開始時の状態と一致しているか
fn matches_character_select(main_menu: &MainMenu, th19: &Th19, init: &GameInitial) -> bool {
    let menu = main_menu.menu();
    let selection = th19.selection();
    menu.p1_cursor().cursor == init.p1().character() as u32
        && menu.p2_cursor().cursor == init.p2().character() as u32
        && selection.p1().card == init.p1().card() as u32
        && selection.p2().card == init.p2().card() as u32
}

fn set_character_select(th19: &mut Th19, init: &GameInitial) {
    let selection = th19.selection_mut();
    selection.p1_mut().card = init.p1().card() as u32;
    selection.p2_mut().card = init.p2().card() as u32;
    let menu = th19
        .app_mut()
        .main_loop_tasks_mut()
        .find_main_menu_mut()
        .unwrap()
        .menu_mut();
    let p1_cursor = menu.p1_cursor_mut();
    p1_cursor.cursor = init.p1().character() as u32;
    p1_cursor.prev_cursor = p1_cursor.cursor;
    let p2_cursor = menu.p2_cursor_mut();
    p2_cursor.cursor = init.p2().character() as u32;
    p2_cursor.prev_cursor = p2_cursor.cursor;
}

/// 試合開始時の状態を受け取ってから、試合開始までに許容するフレーム数
///
/// 状態を合わせられない場合 (カーソルの値とキャラクター番号の対応が想定と違う場合など) に諦める
const MAX_SYNC_FRAMES: u32 = 60 * 60;

/// 難易度選択画面またはキャラクター選択画面で、ホストの試合開始を待つ
///
/// 試合開始時の状態を受け取ったら、難易度・キャラクター・カードを合わせ、
/// 一致していることを確認してから両プレイヤーを決定させる
pub struct SpectatorStandby {
    cursors_reset: bool,
    game_initial: Option<GameInitial>,
    sync_frames: u32,
    /// 直前のフレームで PAUSE を自動で入力したか。観戦者自身の PAUSE と区別するために使う
    pause_injected: bool,
}

impl SpectatorStandby {
    /// `first_time` は観戦を開始した直後かどうか
    pub fn new(first_time: bool) -> Self {
        Self {
            cursors_reset: !first_time,
            game_initial: None,
            sync_frames: 0,
            pause_injected: false,
        }
    }

    pub fn pause_injected(&self) -> bool {
        self.pause_injected
    }

    pub fn is_waiting_for_host(&self) -> bool {
        self.game_initial.is_none()
    }

    /// 試合開始時の状態を返す。読み込み画面に移ったときに取り出す
    pub fn take_game_initial(&mut self) -> Option<GameInitial> {
        self.game_initial.take()
    }

    /// 試合開始時の状態を受信済みなら `true` を返す
    fn recv(&mut self, session: &mut SpectatorSession, th19: &mut Th19) -> Result<bool, RecvError> {
        if self.game_initial.is_some() {
            return Ok(true);
        }
        if session.spectator_initial().is_none() && !session.try_recv_init_spectator()? {
            return Ok(false);
        }
        self.game_initial = session.try_recv_init_game()?;
        if let Some(init) = &self.game_initial {
            trace!("game_initial: {:?}", init);
            // 待機中は負荷を抑えるため通常速度にしていたので、追いつくために早送りする
            th19.set_no_wait(true);
            return Ok(true);
        }
        if th19.no_wait() {
            th19.set_no_wait(false);
        }
        Ok(false)
    }

    pub fn update_th19_on_input_players(
        &mut self,
        session: &mut SpectatorSession,
        main_menu: &MainMenu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if !self.cursors_reset {
            self.cursors_reset = true;
            reset_cursors(th19);
        }
        self.pause_injected = false;
        if !self.recv(session, th19)? {
            clear_player_inputs(th19);
            return Ok(());
        }
        self.sync_frames += 1;
        if self.sync_frames > MAX_SYNC_FRAMES {
            warn!("failed to reproduce game initial");
            return Err(RecvError);
        }
        let init = self.game_initial.clone().unwrap();
        if main_menu.screen_id() != ScreenId::CharacterSelect {
            clear_player_inputs(th19);
            return Ok(());
        }
        let input_devices = th19.input_devices();
        let (prev_p1, prev_p2) = (
            input_devices.p1_input().prev(),
            input_devices.p2_input().prev(),
        );
        let (p1, p2) = if th19.selection().difficulty as u8 != init.difficulty() {
            // 難易度が異なるので難易度選択画面に戻る
            let pause: InputValue = InputFlags::PAUSE.into();
            let p1 = if prev_p1 == pause {
                InputValue::empty()
            } else {
                self.pause_injected = true;
                pause
            };
            (p1, InputValue::empty())
        } else if matches_character_select(main_menu, th19, &init) {
            // 一致していることを確認できたので決定する。準備完了後もカードは変更できるため合わせ続ける
            (shot_repeatedly(prev_p1), shot_repeatedly(prev_p2))
        } else {
            set_character_select(th19, &init);
            (InputValue::empty(), InputValue::empty())
        };
        let input_devices = th19.input_devices_mut();
        input_devices.p1_input_mut().set_current(p1);
        input_devices.p2_input_mut().set_current(p2);
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
        if !self.recv(session, th19)? {
            th19.menu_input_mut().set_current(InputValue::empty());
            return Ok(());
        }
        let init = self.game_initial.clone().unwrap();
        let menu = main_menu.menu_mut();
        if menu.cursor() != init.difficulty() as u32 {
            menu.set_cursor(init.difficulty() as u32);
            th19.menu_input_mut().set_current(InputValue::empty());
            return Ok(());
        }
        // キャラクター選択画面の初期カーソル位置を合わせてから決定する
        let selection = th19.selection_mut();
        selection.p1_mut().character = init.p1().character() as u32;
        selection.p2_mut().character = init.p2().character() as u32;
        let prev = th19.menu_input().prev();
        th19.menu_input_mut().set_current(shot_repeatedly(prev));
        Ok(())
    }
}
