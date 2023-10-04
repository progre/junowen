use std::{
    collections::LinkedList,
    sync::mpsc::{self, RecvError},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

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
    delay: u8,
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
        }
    }

    /// positive value when buffer data is too much,
    /// negative value when buffer data is not enough
    fn delay_gap(&self) -> i8 {
        let current_delay = self
            .local
            .iter()
            .filter(|x| matches!(x, InternalDelayedInput::Input(_)))
            .count() as i32;
        current_delay as i8 - (self.delay as i8)
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
                    trace!("delay gap updated: {}", self.delay_gap());
                    return Ok(round_initial);
                }
            }
        }
        panic!("desync");
    }

    pub fn enqueue_input(&mut self, input: DelayedInput, delay: Option<u8>) {
        if self.delay_gap() > 0 {
            // self.delay_gap() -= 1;
            trace!("delay gap updated: {}", self.delay_gap());
            return;
        }
        if let Some(delay) = delay {
            let _ = self.remote_sender.send(InternalDelayedInput::Delay(delay));
            self.local.push_back(InternalDelayedInput::Delay(delay));
        }
        let DelayedInput::Input(input_u8) = input;
        let _ = self
            .remote_sender
            .send(InternalDelayedInput::Input(input_u8));
        self.local.push_back(input.into());
    }

    pub fn dequeue_inputs(&mut self) -> Result<(DelayedInput, u8), RecvError> {
        // TODO: enqueue したりしなかったりする分でギャップがずれる
        if self.delay_gap() < 0 {
            // self.delay_gap() += 1;
            trace!("delay gap updated: {}", self.delay_gap());
            return Ok((DelayedInput::Input(0), 0));
        }
        let (p1, p2, delay) = if self.host {
            let (local, local_delay) = self.dequeue_local().unwrap();
            let (DelayedInput::Input(remote), _remote_delay) = self.dequeue_remote()?;
            (local, remote, local_delay)
        } else {
            let (remote, remote_delay) = self.dequeue_remote()?;
            let (DelayedInput::Input(local), _local_delay) = self.dequeue_local().unwrap();
            (remote, local, remote_delay)
        };
        if let Some(delay) = delay {
            self.update_delay(delay);
        }
        Ok((p1, p2))
    }

    fn update_delay(&mut self, delay: u8) {
        debug!("delay update: {} -> {}", self.delay, delay);
        self.delay = delay;
        debug!("delay gap={}", self.delay_gap());
    }

    fn dequeue_local(&mut self) -> Option<(DelayedInput, Option<u8>)> {
        let mut delay = None;
        loop {
            let local = self.local.pop_front()?;
            debug_assert!(matches!(local, InternalDelayedInput::Input(_)) || self.host);
            match local {
                InternalDelayedInput::Delay(d) => {
                    delay = Some(d);
                    continue;
                }
                InternalDelayedInput::Input(input) => {
                    return Some((DelayedInput::Input(input), delay))
                }
                InternalDelayedInput::InitMatch(_) | InternalDelayedInput::InitRound(_) => panic!(),
            }
        }
    }

    fn dequeue_remote(&mut self) -> Result<(DelayedInput, Option<u8>), RecvError> {
        if self.remote_round_initial.is_some() {
            return Ok((DelayedInput::Input(0), None));
        }
        let mut delay = None;
        loop {
            let remote = self.remote_receiver.recv()?;
            match remote {
                InternalDelayedInput::Delay(d) => {
                    delay = Some(d);
                    continue;
                }
                InternalDelayedInput::Input(input) => {
                    return Ok((DelayedInput::Input(input), delay))
                }
                InternalDelayedInput::InitMatch(_) => {
                    warn!("MAYBE DESYNC: {:?}", remote)
                }
                InternalDelayedInput::InitRound(round_initial) => {
                    debug_assert!(self.remote_round_initial.is_none());
                    self.remote_round_initial = Some(round_initial);
                    return Ok((DelayedInput::Input(0), None));
                }
            }
        }
    }
}
