use anyhow::Result;
use getset::{CopyGetters, Getters, Setters};
use junowen_lib::connection::{DataChannel, PeerConnection};
use tracing::info;

use super::{
    session_message::RoundInitial,
    spectator::{
        SpectatorHostDelegation, SpectatorInitial, SpectatorRelayRoom, SpectatorSessionMessage,
    },
    to_channel,
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

    pub fn send(&self, msg: SpectatorSessionMessage) -> Result<()> {
        Ok(self.hook_outgoing_tx.send(msg)?)
    }

    pub fn send_delegate_spectator_host(&self, room: Option<SpectatorRelayRoom>) -> Result<()> {
        Ok(self
            .hook_outgoing_tx
            .send(SpectatorSessionMessage::DelegateSpectatorHost(
                SpectatorHostDelegation::new(room),
            ))?)
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
}

impl Drop for SpectatorHostSession {
    fn drop(&mut self) {
        info!("spectator session host closed");
    }
}
