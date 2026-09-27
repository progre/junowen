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

/// キャラクター選択画面のカーソルが試合開始時の状態と一致しているか
fn matches_characters(main_menu: &MainMenu, init: &GameInitial) -> bool {
    let menu = main_menu.menu();
    menu.p1_cursor().cursor == init.p1().character() as u32
        && menu.p2_cursor().cursor == init.p2().character() as u32
}

fn set_characters(th19: &mut Th19, init: &GameInitial) {
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

/// キャラクター選択画面に入ってからカードを切り替え始めるまでのフレーム数
const CARD_MOVE_START_DELAY_FRAMES: u32 = 30;
/// 左右を押す前に低速移動を押したまま待つフレーム数
const CARD_MOVE_INTERVAL_FRAMES: u32 = 4;
/// 左右を押してから、カードが切り替わったかを判定するまでに待つ最大フレーム数。
/// 切り替わらなければ入力が受け付けられなかったとみなして押し直す
const CARD_MOVE_ACCEPT_FRAMES: u32 = 10;

#[derive(Clone, Copy)]
enum CardMoveState {
    /// 低速移動を押したまま待っている
    Waiting(u32),
    /// 左右を押した。`before` は押す前の `selection.card`
    Pressed { before: u32, frames: u32 },
}

/// カードの選択状態はメモリへの書き込みでは反映されないため、
/// 通常の操作と同じく「低速移動を押しながら左右」の入力で切り替える
///
/// カードはタイトル画面で初期位置に戻り、以降は前の試合の選択が引き継がれるので、
/// 観戦者側で現在のカードを把握しておき、目標との差の分だけ入力する。
/// 画面遷移中などは入力が無視されるため、`selection.card` の変化で入力が受け付けられたことを確認し、
/// 変化しなければ押し直す
struct PlayerCardMover {
    current: u8,
    state: CardMoveState,
    retries: u32,
}

impl PlayerCardMover {
    fn new(current: u8) -> Self {
        Self {
            current,
            state: CardMoveState::Waiting(CARD_MOVE_START_DELAY_FRAMES),
            retries: 0,
        }
    }

    /// 1フレーム分の入力を返す。`card` は現在の `selection.card`
    fn next_input(&mut self, target: u8, card: u32) -> InputValue {
        if self.current == target {
            return InputValue::empty();
        }
        let slow: InputValue = InputFlags::SLOW.into();
        match self.state {
            CardMoveState::Waiting(frames) if frames > 0 => {
                self.state = CardMoveState::Waiting(frames - 1);
                slow
            }
            CardMoveState::Waiting(_) => {
                let direction = if self.current < target {
                    InputFlags::RIGHT
                } else {
                    InputFlags::LEFT
                };
                self.state = CardMoveState::Pressed {
                    before: card,
                    frames: 0,
                };
                InputValue(InputFlags::SLOW | direction)
            }
            CardMoveState::Pressed { before, .. } if card != before => {
                // 入力が受け付けられた
                if self.current < target {
                    self.current += 1;
                } else {
                    self.current -= 1;
                }
                self.retries = 0;
                self.state = CardMoveState::Waiting(CARD_MOVE_INTERVAL_FRAMES);
                slow
            }
            CardMoveState::Pressed { frames, .. } if frames >= CARD_MOVE_ACCEPT_FRAMES => {
                // 入力が受け付けられなかったので押し直す
                self.retries += 1;
                if self.retries.is_multiple_of(10) {
                    warn!(
                        "card move is not accepted. retries={}, card={}",
                        self.retries, card
                    );
                }
                self.state = CardMoveState::Waiting(CARD_MOVE_INTERVAL_FRAMES);
                slow
            }
            CardMoveState::Pressed { before, frames } => {
                self.state = CardMoveState::Pressed {
                    before,
                    frames: frames + 1,
                };
                slow
            }
        }
    }
}

struct CardMover([PlayerCardMover; 2]);

impl CardMover {
    fn new(current_cards: [u8; 2]) -> Self {
        Self(current_cards.map(PlayerCardMover::new))
    }

    fn is_done(&self, targets: [u8; 2]) -> bool {
        self.0[0].current == targets[0] && self.0[1].current == targets[1]
    }

    /// 1フレーム分の入力を返す。`cards` は現在の `selection.card`
    fn next_inputs(&mut self, targets: [u8; 2], cards: [u32; 2]) -> [InputValue; 2] {
        let [p1, p2] = &mut self.0;
        [
            p1.next_input(targets[0], cards[0]),
            p2.next_input(targets[1], cards[1]),
        ]
    }
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
    card_mover: CardMover,
}

impl SpectatorStandby {
    /// `first_time` は観戦を開始した直後かどうか。`current_cards` は観戦者側の現在のカード
    pub fn new(first_time: bool, current_cards: [u8; 2]) -> Self {
        Self {
            cursors_reset: !first_time,
            game_initial: None,
            sync_frames: 0,
            pause_injected: false,
            card_mover: CardMover::new(current_cards),
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
            // NOTE: 呼び出し側はエラーでセッションを中断するため、`RecvError` を中断の合図として流用している
            return Err(RecvError);
        }
        let init = self.game_initial.clone().unwrap();
        if main_menu.screen_id() != ScreenId::CharacterSelect {
            clear_player_inputs(th19);
            return Ok(());
        }
        let card_targets = [init.p1().card(), init.p2().card()];
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
        } else if !matches_characters(main_menu, &init) {
            set_characters(th19, &init);
            (InputValue::empty(), InputValue::empty())
        } else if !self.card_mover.is_done(card_targets) {
            let selection = th19.selection();
            let cards = [selection.p1().card, selection.p2().card];
            let [p1, p2] = self.card_mover.next_inputs(card_targets, cards);
            (p1, p2)
        } else {
            // 一致していることを確認できたので決定する
            (shot_repeatedly(prev_p1), shot_repeatedly(prev_p2))
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
