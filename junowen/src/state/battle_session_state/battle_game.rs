use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use getset::{Getters, MutGetters};
use junowen_lib::{structs::input_devices::InputValue, Th19};

use crate::{helper::inputed_number, session::battle::BattleSession};

use super::{spectator_host::SpectatorHostState, utils::init_round};

#[derive(new, Getters, MutGetters)]
pub struct BattleGame {
    #[getset(get = "pub", get_mut = "pub")]
    session: BattleSession,
    #[getset(get = "pub")]
    spectator_host_state: SpectatorHostState,
}

impl BattleGame {
    pub fn inner_state(self) -> (BattleSession, SpectatorHostState) {
        (self.session, self.spectator_host_state)
    }

    pub fn update_th19(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        // -1フレーム目、0フレーム目は複数回呼ばれ、回数が不定なのでスキップする
        if th19.round_frame().unwrap().frame < 1 {
            let input_devices = th19.input_devices_mut();
            input_devices
                .p1_input_mut()
                .set_current(InputValue::empty());
            input_devices
                .p2_input_mut()
                .set_current(InputValue::empty());
            return Ok(());
        }
        let input_devices = th19.input_devices_mut();
        let delay = if self.session.host() {
            inputed_number(input_devices)
        } else {
            None
        };
        let (p1, p2) = self
            .session
            .enqueue_input_and_dequeue(input_devices.p1_input().current().bits() as u16, delay)?;
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());

        self.spectator_host_state
            .update(false, None, th19, &self.session, p1, p2);

        Ok(())
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        init_round(th19, &mut self.session, &mut self.spectator_host_state)
    }
}
