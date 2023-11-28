use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use getset::{Getters, MutGetters};
use junowen_lib::{InputValue, Th19};

use crate::session::spectator::SpectatorSession;

#[derive(new, Getters, MutGetters)]
pub struct SpectatorGame {
    #[getset(get = "pub", get_mut = "pub")]
    session: SpectatorSession,
}

impl SpectatorGame {
    pub fn inner_session(self) -> SpectatorSession {
        self.session
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
        let (p1, p2) = self.session.dequeue_inputs()?;
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());
        Ok(())
    }

    pub fn on_round_over(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let init = self.session.dequeue_init_round()?;
        th19.set_rand_seed1(init.seed1).unwrap();
        th19.set_rand_seed2(init.seed2).unwrap();
        th19.set_rand_seed3(init.seed3).unwrap();
        th19.set_rand_seed4(init.seed4).unwrap();
        Ok(())
    }
}
