use anyhow::Result;
use getset::Getters;
use junowen_lib::{
    Th19,
    structs::{app::MainMenu, selection::Player},
};
use tracing::info;

use crate::{
    session::{
        RoundInitial,
        battle::BattleSession,
        spectator::{GameInitial, PlayerGameInitial, SpectatorInitial},
        spectator_host::SpectatorHostSession,
    },
    signaling::waiting_for_match::WaitingForSpectator,
};

fn current_round_initial(th19: &Th19) -> RoundInitial {
    RoundInitial {
        seed1: th19.rand_seed1().unwrap(),
        seed2: th19.rand_seed2().unwrap(),
        seed3: th19.rand_seed3().unwrap(),
        seed4: th19.rand_seed4().unwrap(),
    }
}

fn create_spectator_initial(battle_session: &BattleSession, th19: &Th19) -> SpectatorInitial {
    let local_player_name = th19.vs_mode().player_name().to_string();
    let remote_player_name = battle_session.remote_player_name().clone();
    let (p1_name, p2_name) = if battle_session.host() {
        (local_player_name, remote_player_name)
    } else {
        (remote_player_name, local_player_name)
    };
    SpectatorInitial::new(
        p1_name,
        p2_name,
        battle_session
            .match_initial()
            .as_ref()
            .unwrap()
            .game_settings
            .clone(),
    )
}

/// 進行中の試合の記録。試合の途中で接続した観戦者は、これを受け取って試合の最初から早送りで追いかける
struct GameLog {
    init: GameInitial,
    messages: Vec<GameLogMessage>,
}

enum GameLogMessage {
    InitRound(RoundInitial),
    Inputs(u16, u16),
}

impl GameLog {
    fn send_to(&self, session: &SpectatorHostSession) -> Result<()> {
        session.send_init_game(self.init.clone())?;
        for msg in &self.messages {
            match msg {
                GameLogMessage::InitRound(round_initial) => {
                    session.send_init_round(round_initial.clone())?
                }
                GameLogMessage::Inputs(p1, p2) => session.send_inputs(*p1, *p2)?,
            }
        }
        Ok(())
    }
}

/// 観戦者の管理
///
/// 観戦者は接続するとプレイヤー名と設定を受け取る。
/// 選択画面で接続した場合は、次の試合の開始時にキャラクター・カード・乱数シードを受け取り、
/// それに合わせて試合を始める。
/// 試合中に接続した場合は、その試合の開始時の状態とそれまでの入力を受け取り、試合の最初から追いかける
#[derive(Getters)]
pub struct SpectatorHostState {
    #[get = "pub"]
    waiting: WaitingForSpectator,
    /// 次の試合の開始を待っている観戦者
    standby_sessions: Vec<SpectatorHostSession>,
    /// 試合を観戦している観戦者
    sessions: Vec<SpectatorHostSession>,
    /// 試合中のみ `Some`。記録は1試合分で、試合ごとに作り直す
    game_log: Option<GameLog>,
}

impl SpectatorHostState {
    pub fn new(waiting: WaitingForSpectator) -> Self {
        Self {
            waiting,
            standby_sessions: Vec::new(),
            sessions: Vec::new(),
            game_log: None,
        }
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
        battle_session: &BattleSession,
    ) {
        let Some(session) = self.waiting.try_recv_session(pushed, main_menu, th19) else {
            return;
        };
        if let Err(err) =
            session.send_init_spectator(create_spectator_initial(battle_session, th19))
        {
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
            game_log.messages.len()
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
            game_log.messages.push(msg);
        }
    }

    /// 試合開始時 (キャラクター選択画面から読み込み画面に移ったとき) に呼ぶ
    pub fn start_game(&mut self, th19: &Th19) {
        let selection = th19.selection();
        let player =
            |player: &Player| PlayerGameInitial::new(player.character as u8, player.card as u8);
        let init = GameInitial::new(
            selection.difficulty as u8,
            player(selection.p1()),
            player(selection.p2()),
            current_round_initial(th19),
        );
        self.sessions.append(&mut self.standby_sessions);
        self.sessions.retain(|session| {
            if let Err(err) = session.send_init_game(init.clone()) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
        self.game_log = Some(GameLog {
            init,
            messages: Vec::new(),
        });
    }

    /// 試合終了時 (試合から選択画面に戻るとき) に呼ぶ
    pub fn end_game(&mut self) {
        self.game_log = None;
    }

    pub fn send_init_round(&mut self, th19: &Th19) {
        let round_initial = current_round_initial(th19);
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
