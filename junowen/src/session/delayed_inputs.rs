use std::{
    collections::LinkedList,
    sync::mpsc::{self, RecvError},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{trace, warn};

use super::{MatchInitial, RoundInitial};

pub enum DelayedInput {
    Input(u8),
}

impl From<DelayedInput> for InternalDelayedInput {
    fn from(value: DelayedInput) -> Self {
        match value {
            DelayedInput::Input(input) => InternalDelayedInput::Input(input),
        }
    }
}

/** input 以外はホストのみ発行できる */
#[derive(Debug, Deserialize, Serialize)]
pub enum InternalDelayedInput {
    Input(u8),
    Delay(u8),
    InitMatch(MatchInitial),
    InitRound(Option<RoundInitial>),
}

pub struct DelayedInputs {
    host: bool,
    local: LinkedList<InternalDelayedInput>,
    remote_sender: mpsc::Sender<InternalDelayedInput>,
    remote_receiver: mpsc::Receiver<InternalDelayedInput>,
    remote_round_initial: Option<Option<RoundInitial>>,
    /** バッファーが多すぎる時はプラス値、バッファーが足りない時はマイナス値 */
    delay: u8,
    delay_gap: i8,
}

impl DelayedInputs {
    pub fn new(
        remote_sender: mpsc::Sender<InternalDelayedInput>,
        remote_receiver: mpsc::Receiver<InternalDelayedInput>,
        host: bool,
        default_delay: u8,
    ) -> Self {
        Self {
            host,
            local: LinkedList::new(),
            remote_sender,
            remote_receiver,
            remote_round_initial: None,
            delay: default_delay,
            delay_gap: -(default_delay as i8),
        }
    }

    pub fn send_init_match(&mut self, init: MatchInitial) {
        let _ = self
            .remote_sender
            .send(InternalDelayedInput::InitMatch(init));
    }

    pub fn recv_init_match(&mut self) -> Result<MatchInitial, RecvError> {
        let msg = self.remote_receiver.recv()?;
        let InternalDelayedInput::InitMatch(init) = msg else {
            panic!("unexpected message: {:?}", msg);
        };
        Ok(init)
    }

    pub fn send_init_round(&mut self, init: Option<RoundInitial>) {
        let _ = self
            .remote_sender
            .send(InternalDelayedInput::InitRound(init));
    }

    pub fn recv_init_round(&mut self) -> Result<Option<RoundInitial>, RecvError> {
        while self.dequeue_local().is_some() {
            trace!("local input skipped");
        }
        if let Some(round_initial) = self.remote_round_initial.take() {
            return Ok(round_initial);
        }
        for _ in 0..10 {
            let msg = self.remote_receiver.recv()?;
            match msg {
                InternalDelayedInput::Delay(delay) => {
                    self.update_delay(delay);
                    continue;
                }
                InternalDelayedInput::Input(_) => {
                    trace!("remote input skipped");
                    continue;
                }
                InternalDelayedInput::InitMatch(_) => panic!("unexpected message: {:?}", msg),
                InternalDelayedInput::InitRound(round_initial) => {
                    self.delay_gap = -(self.delay as i8);
                    return Ok(round_initial);
                }
            }
        }
        panic!("desync");
    }

    pub fn _enqueue_delay(&mut self, delay: u8) {
        self.local.push_back(InternalDelayedInput::Delay(delay));
    }

    pub fn enqueue_local(&mut self, input: DelayedInput) {
        let DelayedInput::Input(input_u8) = input;
        let _ = self
            .remote_sender
            .send(InternalDelayedInput::Input(input_u8));
        self.local.push_back(input.into());
    }

    pub fn dequeue_inputs(&mut self) -> Result<(DelayedInput, u8), RecvError> {
        if self.delay_gap < 0 {
            trace!("current delay gap: {}", self.delay_gap);
            self.delay_gap += 1;
            return Ok((DelayedInput::Input(0), 0));
        }
        loop {
            let (p1, p2) = if self.host {
                let local = self.dequeue_local().unwrap();
                let DelayedInput::Input(remote) = self.dequeue_remote()?;
                (local, remote)
            } else {
                let remote = self.dequeue_remote()?;
                let DelayedInput::Input(local) = self.dequeue_local().unwrap();
                (remote, local)
            };
            if self.delay_gap > 0 {
                self.delay_gap -= 1;
                continue;
            }
            return Ok((p1, p2));
        }
    }

    fn update_delay(&mut self, delay: u8) {
        self.delay = delay;
        let current_delay = (self
            .local
            .iter()
            .filter(|x| matches!(x, InternalDelayedInput::Input(_)))
            .count() as i32)
            - 1; // TODO: -1 であってる？
        self.delay_gap += current_delay as i8 - (delay as i8);
    }

    fn dequeue_local(&mut self) -> Option<DelayedInput> {
        loop {
            let local = self.local.pop_front()?;
            debug_assert!(matches!(local, InternalDelayedInput::Input(_)) || self.host);
            match local {
                InternalDelayedInput::Delay(delay) => {
                    self.update_delay(delay);
                    continue;
                }
                InternalDelayedInput::Input(input) => return Some(DelayedInput::Input(input)),
                InternalDelayedInput::InitMatch(_) | InternalDelayedInput::InitRound(_) => panic!(),
            }
        }
    }

    fn dequeue_remote(&mut self) -> Result<DelayedInput, RecvError> {
        if self.remote_round_initial.is_some() {
            return Ok(DelayedInput::Input(0));
        }
        loop {
            let remote = self.remote_receiver.recv()?;
            match remote {
                InternalDelayedInput::Delay(delay) => {
                    self.update_delay(delay);
                    continue;
                }
                InternalDelayedInput::Input(input) => return Ok(DelayedInput::Input(input)),
                InternalDelayedInput::InitMatch(_) => {
                    warn!("MAYBE DESYNC: {:?}", remote)
                }
                InternalDelayedInput::InitRound(round_initial) => {
                    debug_assert!(self.remote_round_initial.is_none());
                    self.remote_round_initial = Some(round_initial);
                    return Ok(DelayedInput::Input(0));
                }
            }
        }
    }
}
