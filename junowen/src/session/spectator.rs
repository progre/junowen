use std::{
    collections::VecDeque,
    sync::mpsc::{RecvError, TryRecvError},
};

use anyhow::Result;
use derive_new::new;
use getset::{CopyGetters, Getters, Setters};
use junowen_lib::{
    connection::{DataChannel, PeerConnection},
    structs::settings::GameSettings,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use super::{session_message::RoundInitial, to_channel};

#[derive(new, Clone, Debug, Deserialize, Getters, Serialize)]
pub struct SpectatorInitial {
    #[get = "pub"]
    p1_name: String,
    #[get = "pub"]
    p2_name: String,
    #[get = "pub"]
    game_settings: GameSettings,
}

#[derive(new, Clone, Copy, Debug, Deserialize, CopyGetters, PartialEq, Serialize)]
pub struct PlayerGameInitial {
    #[get_copy = "pub"]
    character: u8,
    #[get_copy = "pub"]
    card: u8,
}

/// 試合開始時の状態。観戦者はこれに合わせてキャラクターとカードを決定する
#[derive(new, Clone, Debug, Deserialize, CopyGetters, Getters, Serialize)]
pub struct GameInitial {
    #[get_copy = "pub"]
    difficulty: u8,
    #[get_copy = "pub"]
    p1: PlayerGameInitial,
    #[get_copy = "pub"]
    p2: PlayerGameInitial,
    #[get = "pub"]
    round_initial: RoundInitial,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SpectatorSessionMessage {
    InitSpectator(SpectatorInitial),
    InitGame(GameInitial),
    InitRound(RoundInitial),
    Inputs(u16, u16),
}

#[derive(CopyGetters, Getters, Setters)]
pub struct SpectatorSession {
    _conn: PeerConnection,
    hook_incoming_rx: std::sync::mpsc::Receiver<SpectatorSessionMessage>,
    /// 受信済みで未処理のメッセージ。試合開始がホストより遅れた分を把握するために使う
    buffer: VecDeque<SpectatorSessionMessage>,
    disconnected: bool,
    spectator_initial: Option<SpectatorInitial>,
    round_initial: Option<RoundInitial>,
    /// 観戦者の試合中に受け取った次の試合の開始時の状態
    next_game_initial: Option<GameInitial>,
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
            next_game_initial: None,
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

    /// 観戦者の試合中に次の試合が始まったかどうか
    ///
    /// 観戦者の試合がホストとずれた場合に起こる。この場合は今の試合を打ち切り、次の試合から同期し直す
    pub fn has_next_game_initial(&self) -> bool {
        self.next_game_initial.is_some()
    }

    /// 前の試合で残ったラウンドの初期化情報を破棄する。試合開始を待つ状態に入るときに呼ぶ
    pub fn discard_round_initial(&mut self) {
        self.round_initial = None;
    }

    /// 試合開始時の状態をブロックせずに受信する
    ///
    /// 観戦者の試合がホストより早く終わった場合などに残った入力は読み捨てる
    pub fn try_recv_init_game(&mut self) -> Result<Option<GameInitial>, RecvError> {
        if let Some(init) = self.next_game_initial.take() {
            return Ok(Some(init));
        }
        self.fill_buffer();
        while let Some(msg) = self.buffer.pop_front() {
            match msg {
                SpectatorSessionMessage::InitGame(init) => return Ok(Some(init)),
                SpectatorSessionMessage::InitSpectator(init) => {
                    error!("unexpected init spectator message: {:?}", init);
                    return Err(RecvError);
                }
                msg => warn!("discard message: {:?}", msg),
            }
        }
        if self.disconnected {
            return Err(RecvError);
        }
        Ok(None)
    }

    /// 次の試合が始まっている場合は `None` を返す
    pub fn dequeue_init_round(&mut self) -> Result<Option<RoundInitial>, RecvError> {
        if let Some(round_initial) = self.round_initial.take() {
            return Ok(Some(round_initial));
        }
        if self.next_game_initial.is_some() {
            return Ok(None);
        }
        loop {
            match self.next_message()? {
                SpectatorSessionMessage::InitSpectator(init) => {
                    error!("unexpected init spectator message: {:?}", init);
                    return Err(RecvError);
                }
                SpectatorSessionMessage::InitGame(init) => {
                    warn!("next game started during round over");
                    self.next_game_initial = Some(init);
                    return Ok(None);
                }
                SpectatorSessionMessage::InitRound(round_initial) => {
                    return Ok(Some(round_initial));
                }
                SpectatorSessionMessage::Inputs(..) => continue,
            }
        }
    }

    pub fn dequeue_inputs(&mut self) -> Result<(u16, u16), RecvError> {
        if self.round_initial.is_some() || self.next_game_initial.is_some() {
            return Ok((0, 0));
        }
        match self.next_message()? {
            SpectatorSessionMessage::InitSpectator(init) => {
                error!("unexpected init spectator message: {:?}", init);
                Err(RecvError)
            }
            SpectatorSessionMessage::InitGame(init) => {
                warn!("next game started during game");
                self.next_game_initial = Some(init);
                Ok((0, 0))
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
