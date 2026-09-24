use std::{
    collections::VecDeque,
    sync::mpsc::{RecvError, TryRecvError},
};

use anyhow::Result;
use derive_new::new;
use getset::{CopyGetters, Getters, Setters};
use junowen_lib::{
    connection::{DataChannel, PeerConnection},
    structs::settings::{AbilityCard, GameSettings},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use super::{session_message::RoundInitial, to_channel};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Screen {
    DifficultySelect,
    CharacterSelect,
}

/// キャラクター選択画面での各プレイヤーの進行段階
///
/// メモリ上の値は未解析のため、ホストが決定キーとキャンセルキーの押下から推定する
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum CharacterSelectPhase {
    #[default]
    Character,
    Card,
    Ready,
}

impl CharacterSelectPhase {
    pub fn decided(self, has_card_phase: bool) -> Self {
        match self {
            Self::Character if has_card_phase => Self::Card,
            Self::Character | Self::Card | Self::Ready => Self::Ready,
        }
    }

    pub fn canceled(self, has_card_phase: bool) -> Self {
        match self {
            Self::Ready if has_card_phase => Self::Card,
            Self::Character | Self::Card | Self::Ready => Self::Character,
        }
    }
}

/// カード選択の段階があるかどうか
///
/// TODO: 実機で確認する。`Random` はカードが自動で決まるため選択の段階がないと仮定している
pub fn has_card_select_phase(game_settings: &GameSettings) -> bool {
    matches!(
        game_settings.ability_card(),
        AbilityCard::SelfCard | AbilityCard::AllCard
    )
}

#[derive(new, Clone, Copy, Debug, Deserialize, CopyGetters, Serialize)]
pub struct PlayerInitialState {
    #[get_copy = "pub"]
    character: u8,
    #[get_copy = "pub"]
    card: u8,
    #[get_copy = "pub"]
    phase: CharacterSelectPhase,
}

#[derive(new, Clone, Debug, Deserialize, CopyGetters, Serialize)]
pub struct InitialState {
    #[get_copy = "pub"]
    screen: Screen,
    #[get_copy = "pub"]
    difficulty: u8,
    #[get_copy = "pub"]
    p1: PlayerInitialState,
    #[get_copy = "pub"]
    p2: PlayerInitialState,
}

#[derive(new, Clone, Debug, Deserialize, Getters, Serialize)]
pub struct SpectatorInitial {
    #[get = "pub"]
    p1_name: String,
    #[get = "pub"]
    p2_name: String,
    #[get = "pub"]
    game_settings: GameSettings,
    #[get = "pub"]
    initial_state: InitialState,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SpectatorSessionMessage {
    InitSpectator(SpectatorInitial),
    InitRound(RoundInitial),
    Inputs(u16, u16),
}

#[derive(CopyGetters, Getters, Setters)]
pub struct SpectatorSession {
    _conn: PeerConnection,
    hook_incoming_rx: std::sync::mpsc::Receiver<SpectatorSessionMessage>,
    /// 受信済みで未処理のメッセージ。途中参加時の遅れを把握するために使う
    buffer: VecDeque<SpectatorSessionMessage>,
    disconnected: bool,
    spectator_initial: Option<SpectatorInitial>,
    round_initial: Option<RoundInitial>,
}

impl SpectatorSession {
    pub fn new(conn: PeerConnection, data_channel: DataChannel) -> Self {
        let (_hook_outgoing_tx, hook_incoming_rx) =
            to_channel(data_channel, |input| rmp_serde::from_slice(input));
        Self {
            _conn: conn,
            hook_incoming_rx,
            buffer: VecDeque::new(),
            disconnected: false,
            spectator_initial: None,
            round_initial: None,
        }
    }

    pub fn spectator_initial(&self) -> Option<&SpectatorInitial> {
        self.spectator_initial.as_ref()
    }

    fn fill_buffer(&mut self) {
        loop {
            match self.hook_incoming_rx.try_recv() {
                Ok(msg) => self.buffer.push_back(msg),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.disconnected = true;
                    return;
                }
            }
        }
    }

    fn next_message(&mut self) -> Result<SpectatorSessionMessage, RecvError> {
        if let Some(msg) = self.buffer.pop_front() {
            return Ok(msg);
        }
        self.hook_incoming_rx.recv()
    }

    /// 受信済みで未処理のメッセージ数を返す
    ///
    /// 呼び出し時点で受信できるメッセージをすべてバッファへ取り込む
    pub fn poll_buffered_len(&mut self) -> usize {
        self.fill_buffer();
        self.buffer.len()
    }

    /// ホスト側の準備が整うまで待つ必要があるため、ブロックせずに受信を試みる
    ///
    /// 受信できた場合は `true` を返す
    pub fn try_recv_init_spectator(&mut self) -> Result<bool, RecvError> {
        self.fill_buffer();
        let Some(msg) = self.buffer.pop_front() else {
            return if self.disconnected {
                Err(RecvError)
            } else {
                Ok(false)
            };
        };
        let init = match msg {
            SpectatorSessionMessage::InitSpectator(init) => init,
            msg => {
                error!("unexpected message: {:?}", msg);
                return Err(RecvError);
            }
        };
        self.spectator_initial = Some(init);
        Ok(true)
    }

    pub fn dequeue_init_round(&mut self) -> Result<RoundInitial, RecvError> {
        if let Some(round_initial) = self.round_initial.take() {
            return Ok(round_initial);
        }
        loop {
            match self.next_message()? {
                SpectatorSessionMessage::InitSpectator(init) => {
                    error!("unexpected init spectator message: {:?}", init);
                    return Err(RecvError);
                }
                SpectatorSessionMessage::InitRound(round_initial) => return Ok(round_initial),
                SpectatorSessionMessage::Inputs(..) => continue,
            }
        }
    }

    pub fn dequeue_inputs(&mut self) -> Result<(u16, u16), RecvError> {
        if self.round_initial.is_some() {
            return Ok((0, 0));
        }
        match self.next_message()? {
            SpectatorSessionMessage::InitSpectator(init) => {
                error!("unexpected init spectator message: {:?}", init);
                Err(RecvError)
            }
            SpectatorSessionMessage::InitRound(round_initial) => {
                self.round_initial = Some(round_initial);
                Ok((0, 0))
            }
            SpectatorSessionMessage::Inputs(p1, p2) => Ok((p1, p2)),
        }
    }
}

impl Drop for SpectatorSession {
    fn drop(&mut self) {
        info!("spectator session guest closed");
    }
}
