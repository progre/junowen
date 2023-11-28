use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use getset::{Getters, MutGetters};
use junowen_lib::{InputValue, Th19};

use crate::{
    helper::inputed_number,
    session::{battle::BattleSession, RoundInitial},
};

use super::spectator_host::SpectatorHostState;

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
        if th19.round().unwrap().frame < 1 {
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
        if self.session.host() {
            let init = self.session.init_round(Some(RoundInitial {
                seed1: th19.rand_seed1().unwrap(),
                seed2: th19.rand_seed2().unwrap(),
                seed3: th19.rand_seed3().unwrap(),
                seed4: th19.rand_seed4().unwrap(),
            }))?;
            assert!(init.is_none());
        } else {
            let init = self.session.init_round(None)?.unwrap();
            th19.set_rand_seed1(init.seed1).unwrap();
            th19.set_rand_seed2(init.seed2).unwrap();
            th19.set_rand_seed3(init.seed3).unwrap();
            th19.set_rand_seed4(init.seed4).unwrap();
        }
        Ok(())
    }
}
