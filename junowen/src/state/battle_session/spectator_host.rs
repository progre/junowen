use getset::Getters;
use junowen_lib::{
    Th19,
    structs::{
        app::{MainMenu, ScreenId},
        input_devices::{InputFlags, InputValue},
        selection::Player,
    },
};
use tracing::info;

use crate::{
    session::{
        RoundInitial,
        battle::BattleSession,
        spectator::{
            self, CharacterSelectPhase, InitialState, PlayerInitialState, SpectatorInitial,
            has_card_select_phase,
        },
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

fn pushed(prev: u16, current: u16, flag: InputFlags) -> bool {
    let bits = InputValue::from(flag).bits() as u16;
    prev & bits == 0 && current & bits != 0
}

/// キャラクター選択画面での各プレイヤーの進行段階を、決定キーとキャンセルキーの押下から推定する
#[derive(Default)]
struct CharacterSelectTracker {
    phases: [CharacterSelectPhase; 2],
    prev_inputs: [u16; 2],
}

impl CharacterSelectTracker {
    fn update(&mut self, inputs: [u16; 2], has_card_phase: bool) {
        for ((phase, prev), current) in self.phases.iter_mut().zip(self.prev_inputs).zip(inputs) {
            if pushed(prev, current, InputFlags::SHOT) {
                *phase = phase.decided(has_card_phase);
            } else if pushed(prev, current, InputFlags::BOMB) {
                *phase = phase.canceled(has_card_phase);
            }
        }
        self.prev_inputs = inputs;
    }
}

#[derive(Getters)]
pub struct SpectatorHostState {
    #[get = "pub"]
    waiting: WaitingForSpectator,
    sessions: Vec<SpectatorHostSession>,
    /// 対戦中など、状態を送れない間に接続した観戦者
    pending_sessions: Vec<SpectatorHostSession>,
    character_select: CharacterSelectTracker,
    prev_screen_id: Option<ScreenId>,
}

impl SpectatorHostState {
    pub fn new(waiting: WaitingForSpectator) -> Self {
        Self {
            waiting,
            sessions: Vec::new(),
            pending_sessions: Vec::new(),
            character_select: CharacterSelectTracker::default(),
            prev_screen_id: None,
        }
    }

    pub fn count_spectators(&self) -> usize {
        self.sessions.len()
    }

    pub fn send_init_round_if_connected(&mut self, th19: &Th19) {
        let round_initial = current_round_initial(th19);
        self.sessions.retain(|session| {
            if let Err(err) = session.send_init_round(round_initial.clone()) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
    }

    /// 観戦者に送る現在の状態を作る。状態を送れない画面では `None` を返す
    ///
    /// - 難易度選択画面 (カード選択状態が初期状態の場合のみ)
    /// - キャラクター選択画面
    fn create_initial_state(&self, main_menu: &MainMenu, th19: &Th19) -> Option<InitialState> {
        let selection = th19.selection();
        match main_menu.screen_id() {
            ScreenId::DifficultySelect => {
                let vs_mode = th19.vs_mode();
                if vs_mode.p1_card() != 0 || vs_mode.p2_card() != 0 {
                    return None;
                }
                let player = |player: &Player| {
                    PlayerInitialState::new(
                        player.character as u8,
                        player.card as u8,
                        CharacterSelectPhase::Character,
                    )
                };
                Some(InitialState::new(
                    spectator::Screen::DifficultySelect,
                    main_menu.menu().cursor() as u8,
                    player(selection.p1()),
                    player(selection.p2()),
                ))
            }
            ScreenId::CharacterSelect => {
                // キャラクター選択画面では `selection` のキャラクターが有効でないため、カーソル位置を使う
                let menu = main_menu.menu();
                let [p1_phase, p2_phase] = self.character_select.phases;
                Some(InitialState::new(
                    spectator::Screen::CharacterSelect,
                    selection.difficulty as u8,
                    PlayerInitialState::new(
                        menu.p1_cursor().cursor as u8,
                        selection.p1().card as u8,
                        p1_phase,
                    ),
                    PlayerInitialState::new(
                        menu.p2_cursor().cursor as u8,
                        selection.p2().card as u8,
                        p2_phase,
                    ),
                ))
            }
            _ => None,
        }
    }

    fn create_spectator_initial(
        initial_state: InitialState,
        battle_session: &BattleSession,
        th19: &Th19,
    ) -> SpectatorInitial {
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
            initial_state,
        )
    }

    /// 保留中の観戦者に現在の状態を送り、観戦を開始させる
    fn join_pending_sessions(
        &mut self,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
        battle_session: &BattleSession,
    ) {
        if self.pending_sessions.is_empty() {
            return;
        }
        let Some(initial_state) = main_menu.and_then(|x| self.create_initial_state(x, th19)) else {
            return;
        };
        let spectator_initial = Self::create_spectator_initial(initial_state, battle_session, th19);
        let round_initial = current_round_initial(th19);
        for session in std::mem::take(&mut self.pending_sessions) {
            let result = session
                .send_init_spectator(spectator_initial.clone())
                .and_then(|_| session.send_init_round(round_initial.clone()));
            if let Err(err) = result {
                info!("initialize spectator failed: {:?}", err);
                continue;
            }
            info!("spectator joined. {:?}", spectator_initial.initial_state());
            self.sessions.push(session);
        }
    }

    /// `p1_input` と `p2_input` はこのフレームで適用される入力
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
        if screen_id == Some(ScreenId::CharacterSelect)
            && prev_screen_id != Some(ScreenId::CharacterSelect)
        {
            self.character_select = CharacterSelectTracker::default();
        }

        if let Some(session) = self.waiting.try_recv_session(pushed, main_menu, th19) {
            self.pending_sessions.push(session);
        }
        // 状態はこのフレームの入力を適用する前のものなので、入力の送信より先に送る
        self.join_pending_sessions(main_menu, th19, battle_session);

        self.sessions.retain(|session| {
            if let Err(err) = session.send_inputs(p1_input, p2_input) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });

        if screen_id == Some(ScreenId::CharacterSelect) {
            let game_settings = &battle_session.match_initial().unwrap().game_settings;
            self.character_select
                .update([p1_input, p2_input], has_card_select_phase(game_settings));
        }
    }
}
