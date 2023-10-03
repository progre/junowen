use std::{
    collections::LinkedList,
    sync::mpsc::{self, RecvTimeoutError},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{MatchInitial, RandomNumberInitial};

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
    InitRound(RandomNumberInitial),
}

pub struct DelayedInputs {
    host: bool,
    local: LinkedList<InternalDelayedInput>,
    remote_receiver: mpsc::Receiver<InternalDelayedInput>,
    /** バッファーが多すぎる時はプラス値、バッファーが足りない時はマイナス値 */
    delay: u8,
    delay_gap: i8,
}

impl DelayedInputs {
    pub fn new(
        remote_receiver: mpsc::Receiver<InternalDelayedInput>,
        host: bool,
        default_delay: u8,
    ) -> Self {
        Self {
            host,
            local: LinkedList::new(),
            remote_receiver,
            delay: default_delay,
            delay_gap: -(default_delay as i8),
        }
    }

    pub fn _enqueue_delay(&mut self, delay: u8) {
        self.local.push_back(InternalDelayedInput::Delay(delay));
    }

    pub fn enqueue_local(&mut self, input: DelayedInput) {
        self.local.push_back(input.into());
    }

    pub fn dequeue_inputs(&mut self) -> Result<(DelayedInput, u8), RecvTimeoutError> {
        if self.delay_gap < 0 {
            self.delay_gap += 1;
            return Ok((DelayedInput::Input(0), 0));
        }
        loop {
            let (p1, p2) = if self.host {
                let local = self.dequeue_local();
                let DelayedInput::Input(remote) = self.dequeue_remote()?;
                (local, remote)
            } else {
                let remote = self.dequeue_remote()?;
                let DelayedInput::Input(local) = self.dequeue_local();
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

    fn dequeue_local(&mut self) -> DelayedInput {
        loop {
            let local = self.local.pop_front().unwrap();
            debug_assert!(matches!(local, InternalDelayedInput::Input(_)) || self.host);
            match local {
                InternalDelayedInput::Delay(delay) => {
                    self.update_delay(delay);
                    continue;
                }
                InternalDelayedInput::Input(input) => return DelayedInput::Input(input),
                InternalDelayedInput::InitMatch(_) | InternalDelayedInput::InitRound(_) => panic!(),
            }
        }
    }

    fn dequeue_remote(&mut self) -> Result<DelayedInput, RecvTimeoutError> {
        loop {
            let remote = self.remote_receiver.recv()?;
            debug_assert!(matches!(remote, InternalDelayedInput::Input(_)) || !self.host);
            match remote {
                InternalDelayedInput::Delay(delay) => {
                    self.update_delay(delay);
                    continue;
                }
                InternalDelayedInput::Input(input) => return Ok(DelayedInput::Input(input)),
                InternalDelayedInput::InitMatch(_) | InternalDelayedInput::InitRound(_) => {
                    warn!("MAYBE DESYNC: {:?}", remote)
                }
            }
        }
    }

    pub fn reset_delay(&mut self) {
        self.delay_gap = -(self.delay as i8);
    }

    pub fn internal_dequeue_remote(&self) -> Result<InternalDelayedInput> {
        Ok(self.remote_receiver.recv()?)
    }
}
