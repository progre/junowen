use anyhow::Result;
use getset::{CopyGetters, Getters, Setters};
use junowen_lib::connection::{DataChannel, PeerConnection};
use tracing::info;

use super::{
    spectator::{SpectatorInitial, SpectatorSessionMessage},
    to_channel, session_message::RoundInitial,
};

#[derive(CopyGetters, Getters, Setters)]
pub struct SpectatorHostSession {
    _conn: PeerConnection,
    hook_outgoing_tx: std::sync::mpsc::Sender<SpectatorSessionMessage>,
}

impl SpectatorHostSession {
    pub fn new(conn: PeerConnection, data_channel: DataChannel) -> Self {
        let (hook_outgoing_tx, _hook_incoming_rx) =
            to_channel(data_channel, |input| rmp_serde::from_slice(input));
        Self {
            _conn: conn,
            hook_outgoing_tx,
        }
    }

    pub fn send_init_spectator(&self, init: SpectatorInitial) -> Result<()> {
        Ok(self
            .hook_outgoing_tx
            .send(SpectatorSessionMessage::InitSpectator(init))?)
    }

    pub fn send_init_round(&self, init: RoundInitial) -> Result<()> {
        Ok(self
            .hook_outgoing_tx
            .send(SpectatorSessionMessage::InitRound(init))?)
    }

    pub fn send_inputs(&self, p1_input: u16, p2_input: u16) -> Result<()> {
        Ok(self
            .hook_outgoing_tx
            .send(SpectatorSessionMessage::Inputs(p1_input, p2_input))?)
    }
}

impl Drop for SpectatorHostSession {
    fn drop(&mut self) {
        info!("spectator session host closed");
    }
}
