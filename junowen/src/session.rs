mod delayed_inputs;

use std::sync::mpsc::{self, RecvError};

use anyhow::Result;
use bytes::Bytes;
use getset::CopyGetters;
use junowen_lib::{connection::Connection, GameSettings};
use serde::{Deserialize, Serialize};
use tokio::{spawn, sync::broadcast};
use tracing::{debug, info};

use self::delayed_inputs::{DelayedInput, DelayedInputs, InternalDelayedInput};

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
    pub seed5: u32,
    pub seed6: u32,
    pub seed7: u32,
    pub seed8: u32,
}

pub async fn create_session(mut conn: Connection, delay: Option<u8>) -> Result<Session> {
    let (hook_outgoing_tx, hook_outgoing_rx) = std::sync::mpsc::channel::<InternalDelayedInput>();

    let conn_message_sender = conn.message_sender.clone();
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
            if let Err(err) = conn_message_sender.send(data).await {
                debug!("send hook outgoing msg error: {}", err);
                return;
            }
        }
    });

    let (hook_incoming_tx, hook_incoming_rx) = mpsc::channel();
    spawn(async move {
        loop {
            let Some(data) = conn.recv().await else {
                return;
            };
            let msg: InternalDelayedInput = rmp_serde::from_slice(&data).unwrap();
            if let Err(err) = hook_incoming_tx.send(msg) {
                debug!("send hook incoming msg error: {}", err);
                return;
            }
        }
    });
    let (host, delay) = if let Some(delay) = delay {
        hook_outgoing_tx.send(InternalDelayedInput::Delay(delay))?;
        (true, delay)
    } else {
        let msg = hook_incoming_rx.recv()?;
        let InternalDelayedInput::Delay(delay) = msg else {
            panic!("unexpected message: {:?}", msg);
        };
        (false, delay)
    };
    let (closed_sender, closed_receiver) = broadcast::channel(1);
    Ok(Session {
        host,
        delayed_inputs: DelayedInputs::new(hook_outgoing_tx, hook_incoming_rx, host, delay),
        closed_sender,
        closed_receiver,
    })
}

#[derive(CopyGetters)]
pub struct Session {
    #[getset(get_copy = "pub")]
    host: bool,
    delayed_inputs: DelayedInputs,
    closed_sender: broadcast::Sender<()>,
    closed_receiver: broadcast::Receiver<()>,
}

unsafe impl Send for Session {}
unsafe impl Sync for Session {}

impl Drop for Session {
    fn drop(&mut self) {
        info!("session closed");
        let _ = self.closed_sender.send(());
    }
}

impl Session {
    pub fn subscribe_closed_receiver(&self) -> broadcast::Receiver<()> {
        self.closed_receiver.resubscribe()
    }

    pub fn init_match(
        &mut self,
        init: Option<MatchInitial>,
    ) -> Result<Option<MatchInitial>, RecvError> {
        debug_assert!(self.host == init.is_some());
        if let Some(init) = init {
            self.delayed_inputs.send_init_match(init);
            Ok(None)
        } else {
            Ok(Some(self.delayed_inputs.recv_init_match()?))
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

    pub fn enqueue_input(&mut self, input: u8) {
        self.delayed_inputs
            .enqueue_local(DelayedInput::Input(input));
    }

    pub fn dequeue_inputs(&mut self) -> Result<(u8, u8), RecvError> {
        let (p1, p2) = self.delayed_inputs.dequeue_inputs()?;
        let DelayedInput::Input(p1) = p1;
        Ok((p1, p2))
    }
}
