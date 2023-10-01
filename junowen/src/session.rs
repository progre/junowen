mod delayed_inputs;

use std::sync::mpsc::{self, RecvTimeoutError};

use anyhow::Result;
use bytes::Bytes;
use getset::CopyGetters;
use junowen_lib::{connection::Connection, GameSettings};
use serde::{Deserialize, Serialize};
use tokio::{spawn, sync::broadcast};

use self::delayed_inputs::{DelayedInput, DelayedInputs, InternalDelayedInput};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchInitial {
    pub game_settings: GameSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RandomNumberInitial {
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
            hook_outgoing_rx = reusable;
            let msg = match msg {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("recv hook outgoing msg error: {}", err);
                    return;
                }
            };
            let data = Bytes::from(rmp_serde::to_vec(&msg).unwrap());
            conn_message_sender.send(data).await.unwrap();
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
                eprintln!("send hook incoming msg error {}", err);
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
        message_sender: hook_outgoing_tx,
        host,
        delayed_inputs: DelayedInputs::new(hook_incoming_rx, host, delay),
        closed_sender,
        closed_receiver,
    })
}

#[derive(CopyGetters)]
pub struct Session {
    message_sender: mpsc::Sender<InternalDelayedInput>,
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
        let _ = self.closed_sender.send(());
    }
}

impl Session {
    pub fn subscribe_closed_receiver(&self) -> broadcast::Receiver<()> {
        self.closed_receiver.resubscribe()
    }

    pub fn send_init_match(&mut self, init: MatchInitial) {
        self.message_sender
            .send(InternalDelayedInput::InitMatch(init))
            .unwrap();
    }

    pub fn recv_init_match(&mut self) -> Result<MatchInitial> {
        let msg = self.delayed_inputs.internal_dequeue_remote()?;
        let InternalDelayedInput::InitMatch(init) = msg else {
            panic!("unexpected message: {:?}", msg);
        };
        Ok(init)
    }

    pub fn send_init_random_number(&mut self, init: RandomNumberInitial) {
        self.message_sender
            .send(InternalDelayedInput::InitRound(init))
            .unwrap();
    }

    pub fn recv_init_round(&mut self) -> Result<(RandomNumberInitial, u32)> {
        for i in 0..10 {
            let msg = self.delayed_inputs.internal_dequeue_remote()?;
            let InternalDelayedInput::InitRound(init) = msg else {
                continue;
            };
            self.delayed_inputs.reset_delay();

            return Ok((init, i));
        }
        panic!("maybe desync");
    }

    pub fn enqueue_input(&mut self, input: u8) {
        self.delayed_inputs
            .enqueue_local(DelayedInput::Input(input));
        self.message_sender
            .send(InternalDelayedInput::Input(input))
            .unwrap();
    }

    pub fn dequeue_inputs(&mut self) -> Result<(u8, u8), RecvTimeoutError> {
        let (p1, p2) = self.delayed_inputs.dequeue_inputs()?;
        let DelayedInput::Input(p1) = p1;
        Ok((p1, p2))
    }
}
