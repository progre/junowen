mod delayed_inputs;

use std::sync::mpsc::{self, RecvError};

use anyhow::Result;
use bytes::Bytes;
use getset::{CopyGetters, Getters, Setters};
use junowen_lib::{
    connection::{DataChannel, PeerConnection},
    GameSettings,
};
use serde::{Deserialize, Serialize};
use tokio::spawn;
use tracing::{debug, info};

use self::delayed_inputs::{DelayedInputs, SessionMessage};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchInitial {
    pub game_settings: GameSettings,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RoundInitial {
    pub seed1: u16,
    pub seed2: u16,
    pub seed3: u16,
    pub seed4: u16,
}

pub async fn create_session(
    conn: PeerConnection,
    mut data_channel: DataChannel,
    delay: Option<u8>,
) -> Result<Session> {
    let (hook_outgoing_tx, hook_outgoing_rx) = std::sync::mpsc::channel::<SessionMessage>();
    let data_channel_message_sender = data_channel.message_sender.clone();

    spawn(async move {
        let mut hook_outgoing_rx = hook_outgoing_rx;
        loop {
            let (msg, reusable) =
                tokio::task::spawn_blocking(move || (hook_outgoing_rx.recv(), hook_outgoing_rx))
                    .await
                    .unwrap();
            let msg = match msg {
                Ok(ok) => ok,
                Err(err) => {
                    debug!("recv hook outgoing msg error: {}", err);
                    return;
                }
            };
            hook_outgoing_rx = reusable;
            let data = Bytes::from(rmp_serde::to_vec(&msg).unwrap());
            if let Err(err) = data_channel_message_sender.send(data).await {
                debug!("send hook outgoing msg error: {}", err);
                return;
            }
        }
    });

    let (hook_incoming_tx, hook_incoming_rx) = mpsc::channel();
    spawn(async move {
        loop {
            let Some(data) = data_channel.recv().await else {
                return;
            };
            let msg: SessionMessage = rmp_serde::from_slice(&data).unwrap();
            if let Err(err) = hook_incoming_tx.send(msg) {
                debug!("send hook incoming msg error: {}", err);
                return;
            }
        }
    });
    let (host, delay) = if let Some(delay) = delay {
        hook_outgoing_tx.send(SessionMessage::Delay(delay))?;
        (true, delay)
    } else {
        let msg = hook_incoming_rx.recv()?;
        let SessionMessage::Delay(delay) = msg else {
            panic!("unexpected message: {:?}", msg);
        };
        (false, delay)
    };
    Ok(Session {
        _conn: conn,
        remote_player_name: "".to_owned(),
        host,
        delayed_inputs: DelayedInputs::new(hook_outgoing_tx, hook_incoming_rx, host, delay),
    })
}

#[derive(CopyGetters, Getters, Setters)]
pub struct Session {
    _conn: PeerConnection,
    #[getset(get = "pub", set = "pub")]
    remote_player_name: String,
    #[getset(get_copy = "pub")]
    host: bool,
    delayed_inputs: DelayedInputs,
}

unsafe impl Send for Session {}
unsafe impl Sync for Session {}

impl Drop for Session {
    fn drop(&mut self) {
        info!("session closed");
    }
}

impl Session {
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
