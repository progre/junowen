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
use tracing::{error, info};

use super::{session_message::RoundInitial, to_channel};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Screen {
    DifficultySelect,
    CharacterSelect,
    Game,
}

#[derive(new, Clone, Debug, Deserialize, CopyGetters, Serialize)]
pub struct InitialState {
    #[get_copy = "pub"]
    screen: Screen,
    #[get_copy = "pub"]
    difficulty: u8,
    #[get_copy = "pub"]
    p1_character: u8,
    #[get_copy = "pub"]
    p1_card: u8,
    #[get_copy = "pub"]
    p2_character: u8,
    #[get_copy = "pub"]
    p2_card: u8,
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

/// 観戦ホストが予約部屋で観戦者を待ち受けるための情報
#[derive(new, Clone, Debug, Deserialize, Getters, Serialize)]
pub struct SpectatorRelayRoom {
    #[get = "pub"]
    room_name: String,
    #[get = "pub"]
    key: String,
}

/// 観戦ホストへの任命
#[derive(new, Clone, Debug, Deserialize, Serialize)]
pub struct SpectatorHostDelegation {
    /// 予約部屋で待ち受ける場合の部屋の情報。`None` の場合は Pure P2P で待ち受ける
    room: Option<SpectatorRelayRoom>,
}

impl SpectatorHostDelegation {
    pub fn into_room(self) -> Option<SpectatorRelayRoom> {
        self.room
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SpectatorSessionMessage {
    InitSpectator(SpectatorInitial),
    InitRound(RoundInitial),
    Inputs(u16, u16),
    /// 受信した観戦者を観戦ホストに任命し、以降の観戦者の受け付けと中継を委譲する
    DelegateSpectatorHost(SpectatorHostDelegation),
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
    delegation: Option<SpectatorHostDelegation>,
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
            delegation: None,
        }
    }

    pub fn spectator_initial(&self) -> Option<&SpectatorInitial> {
        self.spectator_initial.as_ref()
    }

    /// 観戦ホストに任命された場合、その任命を返す
    pub fn take_delegation(&mut self) -> Option<SpectatorHostDelegation> {
        self.delegation.take()
    }

    /// 観戦ホストへの任命を処理し、それ以外のメッセージを返す
    fn filter_delegation(
        &mut self,
        msg: SpectatorSessionMessage,
    ) -> Option<SpectatorSessionMessage> {
        match msg {
            SpectatorSessionMessage::DelegateSpectatorHost(delegation) => {
                info!("delegated spectator host");
                self.delegation = Some(delegation);
                None
            }
            msg => Some(msg),
        }
    }

    fn fill_buffer(&mut self) {
        loop {
            match self.hook_incoming_rx.try_recv() {
                Ok(msg) => {
                    if let Some(msg) = self.filter_delegation(msg) {
                        self.buffer.push_back(msg);
                    }
                }
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
        loop {
            let msg = self.hook_incoming_rx.recv()?;
            if let Some(msg) = self.filter_delegation(msg) {
                return Ok(msg);
            }
        }
    }

    /// 受信済みで未処理のメッセージ数
    pub fn buffered_len(&mut self) -> usize {
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
                SpectatorSessionMessage::DelegateSpectatorHost(..) => unreachable!(),
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
            SpectatorSessionMessage::DelegateSpectatorHost(..) => unreachable!(),
        }
    }
}

impl Drop for SpectatorSession {
    fn drop(&mut self) {
        info!("spectator session guest closed");
    }
}
