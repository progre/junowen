mod acceptance;
mod game_log;
mod initial;
mod status_text;

use anyhow::Result;
use junowen_lib::{Th19, structs::app::MainMenu};
use tracing::info;

use crate::{
    session::{RoundInitial, spectator::GameInitial, spectator_host::SpectatorHostSession},
    signaling::waiting_for_match::WaitingForSpectator,
};

use self::{
    acceptance::Acceptance,
    game_log::{GameLog, GameLogMessage},
};
pub use self::{
    initial::{SpectatorInitialSource, current_game_initial, current_round_initial},
    status_text::status_text,
};

/// 観戦者の管理
///
/// 観戦者は接続するとプレイヤー名と設定を受け取る。
/// 選択画面で接続した場合は、次の試合の開始時にキャラクター・カード・乱数シードを受け取り、
/// それに合わせて試合を始める。
/// 試合中に接続した場合は、その試合の開始時の状態とそれまでの入力を受け取り、試合の最初から追いかける
///
/// 対戦者は最初の観戦者のみを受け付け、その観戦者を観戦ホストに任命して以降の受け付けを委譲する。
/// 観戦ホストは自分が処理した試合の進行を、同じ順序で配下の観戦者に中継する。
/// 観戦ホストが離脱すると、配下の観戦者もすべて離脱する
pub struct SpectatorHostState {
    acceptance: Acceptance,
    /// 次の試合の開始を待っている観戦者
    standby_sessions: Vec<SpectatorHostSession>,
    /// 試合を観戦している観戦者
    sessions: Vec<SpectatorHostSession>,
    /// 試合中のみ `Some`。記録は1試合分で、試合ごとに作り直す
    game_log: Option<GameLog>,
}

impl SpectatorHostState {
    /// 対戦者用。最初の観戦者を観戦ホストに任命する
    pub fn new(waiting: WaitingForSpectator) -> Self {
        Self::with_acceptance(Acceptance::WaitingForSpectatorHost(waiting))
    }

    /// 観戦ホスト用
    pub fn new_relay(waiting: WaitingForSpectator) -> Self {
        Self::with_acceptance(Acceptance::Relaying(waiting))
    }

    fn with_acceptance(acceptance: Acceptance) -> Self {
        Self {
            acceptance,
            standby_sessions: Vec::new(),
            sessions: Vec::new(),
            game_log: None,
        }
    }

    pub fn waiting(&self) -> Option<&WaitingForSpectator> {
        self.acceptance.waiting()
    }

    pub fn count_spectators(&self) -> usize {
        self.standby_sessions.len() + self.sessions.len()
    }

    /// 観戦者の接続を受け付ける
    pub fn update(
        &mut self,
        pushed: bool,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
        initial_source: &impl SpectatorInitialSource,
    ) {
        let has_spectators = self.count_spectators() > 0;
        let Some(session) =
            self.acceptance
                .try_recv_session(has_spectators, pushed, main_menu, th19)
        else {
            return;
        };
        if let Err(err) = session.send_init_spectator(initial_source.spectator_initial(th19)) {
            info!("initialize spectator failed: {:?}", err);
            return;
        }
        let Some(game_log) = &self.game_log else {
            info!("spectator joined. waiting for next game");
            self.standby_sessions.push(session);
            return;
        };
        if let Err(err) = game_log.send_to(&session) {
            info!("initialize spectator failed: {:?}", err);
            return;
        }
        info!(
            "spectator joined during game. replaying {} messages",
            game_log.message_count()
        );
        self.sessions.push(session);
    }

    fn broadcast(
        &mut self,
        msg: GameLogMessage,
        send: impl Fn(&SpectatorHostSession) -> Result<()>,
    ) {
        self.sessions.retain(|session| {
            if let Err(err) = send(session) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
        if let Some(game_log) = &mut self.game_log {
            game_log.push(msg);
        }
    }

    /// 試合開始時 (キャラクター選択画面から読み込み画面に移ったとき) に呼ぶ
    pub fn start_game(&mut self, init: GameInitial) {
        self.sessions.append(&mut self.standby_sessions);
        self.sessions.retain(|session| {
            if let Err(err) = session.send_init_game(init.clone()) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
        self.game_log = Some(GameLog::new(init));
    }

    /// 試合終了時 (試合から選択画面に戻るとき) に呼ぶ
    pub fn end_game(&mut self) {
        self.game_log = None;
    }

    pub fn send_init_round(&mut self, round_initial: RoundInitial) {
        self.broadcast(
            GameLogMessage::InitRound(round_initial.clone()),
            |session| session.send_init_round(round_initial.clone()),
        );
    }

    pub fn send_inputs(&mut self, p1_input: u16, p2_input: u16) {
        self.broadcast(GameLogMessage::Inputs(p1_input, p2_input), |session| {
            session.send_inputs(p1_input, p2_input)
        });
    }
}
