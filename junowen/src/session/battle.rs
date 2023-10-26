use std::sync::mpsc::RecvError;

use anyhow::Result;
use getset::{CopyGetters, Getters, Setters};
use junowen_lib::{
    connection::{DataChannel, PeerConnection},
    GameSettings,
};
use serde::{Deserialize, Serialize};
use tracing::{info, trace};

use super::{delayed_inputs::DelayedInputs, to_channel, RoundInitial};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchInitial {
    pub game_settings: GameSettings,
}

#[derive(CopyGetters, Getters, Setters)]
pub struct BattleSession {
    _conn: PeerConnection,
    #[getset(get = "pub", set = "pub")]
    remote_player_name: String,
    #[getset(get_copy = "pub")]
    host: bool,
    delayed_inputs: DelayedInputs,
    #[getset(set = "pub")]
    match_initial: Option<MatchInitial>,
}

impl Drop for BattleSession {
    fn drop(&mut self) {
        info!("session closed");
    }
}

impl BattleSession {
    pub fn new(conn: PeerConnection, data_channel: DataChannel, host: bool) -> Self {
        let (hook_outgoing_tx, hook_incoming_rx) =
            to_channel(data_channel, |input| rmp_serde::from_slice(input));
        Self {
            _conn: conn,
            remote_player_name: "".to_owned(),
            host,
            delayed_inputs: DelayedInputs::new(hook_outgoing_tx, hook_incoming_rx, host),
            match_initial: None,
        }
    }

    pub fn match_initial(&self) -> Option<&MatchInitial> {
        self.match_initial.as_ref()
    }

    pub fn delay(&self) -> u8 {
        self.delayed_inputs.delay()
    }

    pub fn init_match(
        &mut self,
        player_name: String,
        init: Option<MatchInitial>,
    ) -> Result<(String, Option<MatchInitial>), RecvError> {
        debug_assert!(self.host == init.is_some());
        if let Some(init) = init {
            self.delayed_inputs
                .send_init_match((player_name, Some(init)));
            self.delayed_inputs.recv_init_match()
        } else {
            self.delayed_inputs.send_init_match((player_name, None));
            self.delayed_inputs.recv_init_match()
        }
    }

    pub fn init_round(
        &mut self,
        init: Option<RoundInitial>,
    ) -> Result<Option<RoundInitial>, RecvError> {
        debug_assert!(self.host == init.is_some());
        trace!("init_round");
        self.delayed_inputs.send_init_round(init);
        self.delayed_inputs.recv_init_round()
    }

    pub fn enqueue_input_and_dequeue(
        &mut self,
        input: u16,
        delay: Option<u8>,
    ) -> Result<(u16, u16), RecvError> {
        self.delayed_inputs.enqueue_input_and_dequeue(input, delay)
    }
}
