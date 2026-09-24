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

    /// 試合開始時の状態をブロックせずに受信する
    ///
    /// 観戦者の試合がホストより早く終わった場合などに残った入力は読み捨てる
    pub fn try_recv_init_game(&mut self) -> Result<Option<GameInitial>, RecvError> {
        self.round_initial = None;
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
                SpectatorSessionMessage::InitGame(init) => {
                    error!("unexpected init game message: {:?}", init);
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
            SpectatorSessionMessage::InitGame(init) => {
                error!("unexpected init game message: {:?}", init);
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
