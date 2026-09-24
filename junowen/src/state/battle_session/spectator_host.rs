use anyhow::Result;
use junowen_lib::{
    Th19,
    structs::app::{MainMenu, ScreenId},
    structs::selection::Selection,
    structs::settings::GameSettings,
};
use tracing::info;

use crate::{
    session::{
        RoundInitial,
        battle::BattleSession,
        spectator::{
            self, InitialState, SpectatorInitial, SpectatorRelayRoom, SpectatorSessionMessage,
        },
        spectator_host::SpectatorHostSession,
    },
    signaling::waiting_for_match::WaitingForSpectator,
};

/// 観戦者に送る対戦の情報
pub struct SpectatorMatchInfo<'a> {
    p1_name: &'a str,
    p2_name: &'a str,
    game_settings: &'a GameSettings,
}

/// 観戦者に送る対戦の情報の取得元
pub trait SpectatorMatchSource {
    fn match_info<'a>(&'a self, th19: &'a Th19) -> SpectatorMatchInfo<'a>;
}

impl SpectatorMatchSource for BattleSession {
    fn match_info<'a>(&'a self, th19: &'a Th19) -> SpectatorMatchInfo<'a> {
        let local_player_name = th19.vs_mode().player_name();
        let remote_player_name = self.remote_player_name().as_str();
        let (p1_name, p2_name) = if self.host() {
            (local_player_name, remote_player_name)
        } else {
            (remote_player_name, local_player_name)
        };
        SpectatorMatchInfo {
            p1_name,
            p2_name,
            game_settings: &self.match_initial().as_ref().unwrap().game_settings,
        }
    }
}

impl SpectatorMatchSource for SpectatorInitial {
    fn match_info<'a>(&'a self, _th19: &'a Th19) -> SpectatorMatchInfo<'a> {
        SpectatorMatchInfo {
            p1_name: self.p1_name(),
            p2_name: self.p2_name(),
            game_settings: self.game_settings(),
        }
    }
}

fn create_spectator_initial(
    current_screen: ScreenId,
    selection: &Selection,
    match_info: SpectatorMatchInfo,
) -> SpectatorInitial {
    SpectatorInitial::new(
        match_info.p1_name.to_owned(),
        match_info.p2_name.to_owned(),
        match_info.game_settings.clone(),
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

/// 観戦者の途中参加用に、直近の同期ポイントの状態とそれ以降のメッセージを保持する
///
/// 途中参加した観戦者は同期ポイントの状態から記録済みのメッセージを早送りで再生し、
/// ホストに追いつく
///
/// `messages` には毎フレームの入力が蓄積され続けるが、同期ポイントはキャラクター選択画面に
/// 入るたびに作り直されるため、蓄積量はおおむね 1 対戦分 (1 分あたり約 3,600 件) に収まる
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

/// 観戦者の受け付けと入力の送信を行う
///
/// 対戦者は最初の観戦者のみを受け付け、その観戦者を観戦ホストに任命して以降の受け付けを委譲する。
/// 観戦ホストは受け付けた観戦者に入力を中継する。
/// 観戦ホストが離脱すると、その観戦者もすべて離脱する。
/// 予約部屋の待ち受けは一度しか受信できないため、受信後に作り直す
fn rearm(waiting: &mut WaitingForSpectator, room: Option<SpectatorRelayRoom>) {
    if room.is_some() {
        *waiting = WaitingForSpectator::new(room);
    }
}

pub struct SpectatorHostState {
    acceptance: Acceptance,
    sessions: Vec<SpectatorHostSession>,
    sync_point: Option<SyncPoint>,
    /// 同期ポイントがまだ無い間に接続した観戦者
    pending_sessions: Vec<SpectatorHostSession>,
    prev_screen_id: Option<ScreenId>,
}

/// 観戦者の受け付け状態
enum Acceptance {
    /// 対戦者: 最初の観戦者を待ち受け、観戦ホストに任命する
    WaitingForSpectatorHost(WaitingForSpectator),
    /// 対戦者: 観戦ホストに受け付けを委譲している
    Delegated,
    /// 観戦ホスト: 観戦者を待ち受け、入力を中継する
    Relaying(WaitingForSpectator),
}

impl SpectatorHostState {
    /// 対戦者用
    pub fn new(waiting: WaitingForSpectator) -> Self {
        Self::internal_new(Acceptance::WaitingForSpectatorHost(waiting))
    }

    /// 観戦ホスト用
    pub fn new_relay(waiting: WaitingForSpectator) -> Self {
        Self::internal_new(Acceptance::Relaying(waiting))
    }

    fn internal_new(acceptance: Acceptance) -> Self {
        Self {
            acceptance,
            sessions: Vec::new(),
            sync_point: None,
            pending_sessions: Vec::new(),
            prev_screen_id: None,
        }
    }

    pub fn waiting(&self) -> Option<&WaitingForSpectator> {
        match &self.acceptance {
            Acceptance::WaitingForSpectatorHost(waiting) | Acceptance::Relaying(waiting) => {
                Some(waiting)
            }
            Acceptance::Delegated => None,
        }
    }

    pub fn count_spectators(&self) -> usize {
        self.sessions.len()
    }

    pub fn count_pending_spectators(&self) -> usize {
        self.pending_sessions.len()
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
            sync_point.messages.push(msg);
        }
    }

    pub fn send_init_round_if_connected(&mut self, th19: &Th19) {
        self.broadcast(SpectatorSessionMessage::InitRound(current_round_initial(
            th19,
        )));
    }

    fn join(&mut self, session: SpectatorHostSession) {
        let Some(sync_point) = &self.sync_point else {
            info!("spectator connected. waiting for sync point");
            self.pending_sessions.push(session);
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

    fn recv_session(&mut self, pushed: bool, main_menu: Option<&MainMenu>, th19: &Th19) {
        let (waiting, delegates) = match &mut self.acceptance {
            Acceptance::WaitingForSpectatorHost(waiting) => (waiting, true),
            Acceptance::Relaying(waiting) => (waiting, false),
            Acceptance::Delegated => {
                if self.sessions.is_empty() && self.pending_sessions.is_empty() {
                    // 観戦ホストが離脱したので受け付けを再開する。
                    // 予約部屋は観戦ホストの離脱時に削除されるため、Pure P2P で待ち受ける
                    info!("spectator host left. resume waiting for spectator");
                    self.acceptance =
                        Acceptance::WaitingForSpectatorHost(WaitingForSpectator::new(None));
                }
                return;
            }
        };
        let Some(session) = waiting.try_recv_session(pushed, main_menu, th19) else {
            return;
        };
        let room = waiting.room().cloned();
        if delegates {
            if let Err(err) = session.send_delegate_spectator_host(room.clone()) {
                info!("delegate spectator host failed: {:?}", err);
                rearm(waiting, room);
                return;
            }
            // 予約部屋の待ち受けは観戦ホストが引き継ぐため、ここでは作り直さない
            // (作り直した待ち受けを破棄すると部屋が削除されてしまう)
            info!("delegated spectator host");
            self.acceptance = Acceptance::Delegated;
        } else {
            rearm(waiting, room);
        }
        self.join(session);
    }

    pub fn update(
        &mut self,
        pushed: bool,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
        match_source: &impl SpectatorMatchSource,
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
                    match_source.match_info(th19),
                ),
                round_initial: current_round_initial(th19),
                messages: Vec::new(),
            });
            for session in std::mem::take(&mut self.pending_sessions) {
                self.join(session);
            }
        }

        self.recv_session(pushed, main_menu, th19);

        self.broadcast(SpectatorSessionMessage::Inputs(p1_input, p2_input));
    }
}
