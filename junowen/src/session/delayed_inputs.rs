use std::{
    collections::LinkedList,
    sync::mpsc::{self, RecvError},
};

use anyhow::Result;
use getset::CopyGetters;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use super::{MatchInitial, RoundInitial};

/** input 以外はホストのみ発行できる */
#[derive(Debug, Deserialize, Serialize)]
pub enum InternalDelayedInput {
    Input(u16),
    Delay(u8),
    InitMatch((String, Option<MatchInitial>)),
    InitRound(Option<RoundInitial>),
}

#[derive(CopyGetters)]
pub struct DelayedInputs {
    host: bool,
    local: LinkedList<InternalDelayedInput>,
    remote_sender: mpsc::Sender<InternalDelayedInput>,
    remote_receiver: mpsc::Receiver<InternalDelayedInput>,
    remote_round_initial: Option<Option<RoundInitial>>,
    #[getset(get_copy = "pub")]
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

    pub fn send_init_match(&mut self, init: (String, Option<MatchInitial>)) {
        let _ = self
            .remote_sender
            .send(InternalDelayedInput::InitMatch(init));
    }

    pub fn recv_init_match(&mut self) -> Result<(String, Option<MatchInitial>), RecvError> {
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
        let mut local_delay = None;
        loop {
            let Some((_, delay)) = self.dequeue_local() else {
                break;
            };
            if let Some(delay) = delay {
                debug_assert!(self.host);
                local_delay = Some(delay);
            }
            trace!("local input skipped");
        }
        let mut remote_delay = None;
        let round_initial = loop {
            if let Some(round_initial) = self.remote_round_initial.take() {
                break round_initial;
            }
            let (_, delay) = self.dequeue_remote()?;
            if let Some(delay) = delay {
                debug_assert!(self.host);
                remote_delay = Some(delay);
            }
            trace!("remote input skipped");
        };
        let delay = if self.host { local_delay } else { remote_delay };
        if let Some(delay) = delay {
            self.update_delay(delay);
        }
        Ok(round_initial)
    }

    pub fn enqueue_input_and_dequeue(
        &mut self,
        input: u16,
        delay: Option<u8>,
    ) -> Result<(u16, u16), RecvError> {
        let delay_gap = self.delay_gap();
        if delay_gap <= 0 {
            if let Some(delay) = delay {
                let _ = self.remote_sender.send(InternalDelayedInput::Delay(delay));
                self.local.push_back(InternalDelayedInput::Delay(delay));
            }
            let _ = self.remote_sender.send(InternalDelayedInput::Input(input));
            self.local.push_back(InternalDelayedInput::Input(input));
        }
        if delay_gap < 0 {
            trace!("delay gap updated: {}", self.delay_gap());
            return Ok((0, 0));
        }
        let (local, local_delay) = self.dequeue_local().unwrap();
        let (remote, remote_delay) = self.dequeue_remote()?;
        let (p1, p2, delay) = if self.host {
            (local, remote, local_delay)
        } else {
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

    fn dequeue_local(&mut self) -> Option<(u16, Option<u8>)> {
        let mut delay = None;
        loop {
            let local = self.local.pop_front()?;
            debug_assert!(matches!(local, InternalDelayedInput::Input(_)) || self.host);
            match local {
                InternalDelayedInput::InitMatch(_) => panic!("unexpected message: {:?}", local),
                InternalDelayedInput::Delay(d) => {
                    debug_assert!(self.host);
                    delay = Some(d);
                    continue;
                }
                InternalDelayedInput::Input(input) => return Some((input, delay)),
                InternalDelayedInput::InitRound(_) => panic!(),
            }
        }
    }

    fn dequeue_remote(&mut self) -> Result<(u16, Option<u8>), RecvError> {
        if self.remote_round_initial.is_some() {
            return Ok((0, None));
        }
        let mut delay = None;
        loop {
            let remote = self.remote_receiver.recv()?;
            match remote {
                InternalDelayedInput::InitMatch(_) => panic!("unexpected message: {:?}", remote),
                InternalDelayedInput::Delay(d) => {
                    debug_assert!(!self.host);
                    delay = Some(d);
                    continue;
                }
                InternalDelayedInput::Input(input) => return Ok((input, delay)),
                InternalDelayedInput::InitRound(round_initial) => {
                    debug_assert!(self.remote_round_initial.is_none());
                    self.remote_round_initial = Some(round_initial);
                    return Ok((0, None));
                }
            }
        }
    }
}
