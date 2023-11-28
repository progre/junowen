use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use getset::{CopyGetters, Getters, Setters};
use junowen_lib::{
    connection::{DataChannel, PeerConnection},
    GameSettings,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use super::{to_channel, RoundInitial};

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
            spectator_initial: None,
            round_initial: None,
        }
    }

    pub fn spectator_initial(&self) -> Option<&SpectatorInitial> {
        self.spectator_initial.as_ref()
    }

    pub fn recv_init_spectator(&mut self) -> Result<(), RecvError> {
        let init = match self.hook_incoming_rx.recv()? {
            SpectatorSessionMessage::InitSpectator(init) => init,
            msg => {
                error!("unexpected message: {:?}", msg);
                return Err(RecvError);
            }
        };
        self.spectator_initial = Some(init);
        Ok(())
    }

    pub fn dequeue_init_round(&mut self) -> Result<RoundInitial, RecvError> {
        if let Some(round_initial) = self.round_initial.take() {
            return Ok(round_initial);
        }
        loop {
            match self.hook_incoming_rx.recv()? {
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
        match self.hook_incoming_rx.recv()? {
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
