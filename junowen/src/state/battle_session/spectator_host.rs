use anyhow::Result;
use getset::Getters;
use junowen_lib::{
    Th19,
    structs::app::{MainMenu, ScreenId},
    structs::selection::Selection,
};
use tracing::info;

use crate::{
    session::{
        RoundInitial,
        battle::BattleSession,
        spectator::{self, InitialState, SpectatorInitial, SpectatorSessionMessage},
        spectator_host::SpectatorHostSession,
    },
    signaling::waiting_for_match::WaitingForSpectator,
};

fn create_spectator_initial(
    current_screen: ScreenId,
    selection: &Selection,
    battle_session: &BattleSession,
    local_player_name: String,
) -> SpectatorInitial {
    let p1_name = if battle_session.host() {
        local_player_name.to_owned()
    } else {
        battle_session.remote_player_name().clone()
    };
    let p2_name = if battle_session.host() {
        battle_session.remote_player_name().clone()
    } else {
        local_player_name.to_owned()
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
        InitialState::new(
            match current_screen {
                ScreenId::DifficultySelect => spectator::Screen::DifficultySelect,
                ScreenId::CharacterSelect => spectator::Screen::CharacterSelect,
                _ => unreachable!(),
            },
            selection.difficulty as u8,
            selection.p1().character as u8,
            selection.p1().card as u8,
            selection.p2().character as u8,
            selection.p2().card as u8,
        ),
    )
}

fn current_round_initial(th19: &Th19) -> RoundInitial {
    RoundInitial {
        seed1: th19.rand_seed1().unwrap(),
        seed2: th19.rand_seed2().unwrap(),
        seed3: th19.rand_seed3().unwrap(),
        seed4: th19.rand_seed4().unwrap(),
    }
}

/// 同期ポイントにできる画面かどうかを判定する
///
/// 画面に入った最初のフレームのみ同期ポイントにできる
/// - 難易度選択画面 (カード選択状態が初期状態の場合のみ)
/// - キャラクター選択画面
fn is_sync_point(screen_id: ScreenId, th19: &Th19) -> bool {
    match screen_id {
        ScreenId::DifficultySelect => {
            let vs_mode = th19.vs_mode();
            vs_mode.p1_card() == 0 && vs_mode.p2_card() == 0
        }
        ScreenId::CharacterSelect => true,
        _ => false,
    }
}

/// 同期ポイント以降に記録するメッセージ数の上限 (60fps で約 10 分)
///
/// `messages` には毎フレームの入力が記録される。キャラクター選択画面で待機している間も
/// 増え続けるため、上限を超えたら同期ポイントを破棄し、次の同期ポイントまで途中参加を受け付けない。
/// 記録量が多いほど、途中参加時に送る量と観戦者が追いつくまでの時間も増える
const MAX_SYNC_POINT_MESSAGES: usize = 60 * 60 * 10;

/// 観戦者の途中参加用に、直近の同期ポイントの状態とそれ以降のメッセージを保持する
///
/// 途中参加した観戦者は同期ポイントの状態から記録済みのメッセージを早送りで再生し、
/// ホストに追いつく
struct SyncPoint {
    spectator_initial: SpectatorInitial,
    round_initial: RoundInitial,
    messages: Vec<SpectatorSessionMessage>,
}

impl SyncPoint {
    fn send_to(&self, session: &SpectatorHostSession) -> Result<()> {
        session.send_init_spectator(self.spectator_initial.clone())?;
        session.send_init_round(self.round_initial.clone())?;
        for msg in &self.messages {
            session.send(msg.clone())?;
        }
        Ok(())
    }
}

#[derive(Getters)]
pub struct SpectatorHostState {
    #[get = "pub"]
    waiting: WaitingForSpectator,
    sessions: Vec<SpectatorHostSession>,
    /// 途中参加を受け付けられない間は `None`
    sync_point: Option<SyncPoint>,
    prev_screen_id: Option<ScreenId>,
}

impl SpectatorHostState {
    pub fn new(waiting: WaitingForSpectator) -> Self {
        Self {
            waiting,
            sessions: Vec::new(),
            sync_point: None,
            prev_screen_id: None,
        }
    }

    pub fn count_spectators(&self) -> usize {
        self.sessions.len()
    }

    fn broadcast(&mut self, msg: SpectatorSessionMessage) {
        self.sessions.retain(|session| {
            if let Err(err) = session.send(msg.clone()) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
        if let Some(sync_point) = &mut self.sync_point {
            if sync_point.messages.len() >= MAX_SYNC_POINT_MESSAGES {
                info!(
                    "too many messages since sync point. spectators cannot join until next sync point"
                );
                self.sync_point = None;
            } else {
                sync_point.messages.push(msg);
            }
        }
    }

    pub fn send_init_round_if_connected(&mut self, th19: &Th19) {
        self.broadcast(SpectatorSessionMessage::InitRound(current_round_initial(
            th19,
        )));
    }

    fn join(&mut self, session: SpectatorHostSession) {
        let Some(sync_point) = &self.sync_point else {
            // 最初の同期ポイントは難易度選択画面の最初のフレームで作られるため、
            // ここに来るのは記録量が上限を超えた場合のみ
            info!("spectator rejected. no sync point");
            return;
        };
        if let Err(err) = sync_point.send_to(&session) {
            info!("initialize spectator failed: {:?}", err);
            return;
        }
        info!(
            "spectator joined. replaying {} messages",
            sync_point.messages.len()
        );
        self.sessions.push(session);
    }

    pub fn update(
        &mut self,
        pushed: bool,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
        battle_session: &BattleSession,
        p1_input: u16,
        p2_input: u16,
    ) {
        let screen_id = main_menu.map(|x| x.screen_id());
        let prev_screen_id = std::mem::replace(&mut self.prev_screen_id, screen_id);
        if let Some(screen_id) = screen_id
            && Some(screen_id) != prev_screen_id
            && is_sync_point(screen_id, th19)
        {
            self.sync_point = Some(SyncPoint {
                spectator_initial: create_spectator_initial(
                    screen_id,
                    th19.selection(),
                    battle_session,
                    th19.vs_mode().player_name().to_string(),
                ),
                round_initial: current_round_initial(th19),
                messages: Vec::new(),
            });
        }

        if let Some(session) = self.waiting.try_recv_session(pushed, main_menu, th19) {
            self.join(session);
        }

        self.broadcast(SpectatorSessionMessage::Inputs(p1_input, p2_input));
    }
}
