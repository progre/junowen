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

/// 観戦者の管理
///
/// 観戦者は接続するとプレイヤー名と設定を受け取り、次の試合の開始を待つ。
/// 試合開始時にキャラクター・カード・乱数シードを受け取り、それに合わせて試合を始める
#[derive(Getters)]
pub struct SpectatorHostState {
    #[get = "pub"]
    waiting: WaitingForSpectator,
    /// 次の試合の開始を待っている観戦者
    standby_sessions: Vec<SpectatorHostSession>,
    /// 試合を観戦している観戦者
    sessions: Vec<SpectatorHostSession>,
}

impl SpectatorHostState {
    pub fn new(waiting: WaitingForSpectator) -> Self {
        Self {
            waiting,
            standby_sessions: Vec::new(),
            sessions: Vec::new(),
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
        info!("spectator joined. waiting for next game");
        self.standby_sessions.push(session);
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
    }

    pub fn send_init_round(&mut self, th19: &Th19) {
        let round_initial = current_round_initial(th19);
        self.sessions.retain(|session| {
            if let Err(err) = session.send_init_round(round_initial.clone()) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
    }

    pub fn send_inputs(&mut self, p1_input: u16, p2_input: u16) {
        self.sessions.retain(|session| {
            if let Err(err) = session.send_inputs(p1_input, p2_input) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
    }
}
