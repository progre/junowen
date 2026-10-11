use anyhow::Result;

use crate::session::{RoundInitial, spectator::GameInitial, spectator_host::SpectatorHostSession};

/// 進行中の試合の記録。試合の途中で接続した観戦者は、これを受け取って試合の最初から早送りで追いかける
pub struct GameLog {
    init: GameInitial,
    messages: Vec<GameLogMessage>,
}

pub enum GameLogMessage {
    InitRound(RoundInitial),
    Inputs(u16, u16),
}

impl GameLog {
    pub fn new(init: GameInitial) -> Self {
        Self {
            init,
            messages: Vec::new(),
        }
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn push(&mut self, msg: GameLogMessage) {
        self.messages.push(msg);
    }

    pub fn send_to(&self, session: &SpectatorHostSession) -> Result<()> {
        session.send_init_game(self.init.clone())?;
        for msg in &self.messages {
            match msg {
                GameLogMessage::InitRound(round_initial) => {
                    session.send_init_round(round_initial.clone())?
                }
                GameLogMessage::Inputs(p1, p2) => session.send_inputs(*p1, *p2)?,
            }
        }
        Ok(())
    }
}
