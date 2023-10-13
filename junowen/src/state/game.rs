use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::{InputValue, Th19};

use crate::{
    helper::inputed_number,
    session::{RoundInitial, Session},
};

pub fn on_input_players(session: &mut Session, th19: &mut Th19) -> Result<(), RecvError> {
    // -1フレーム目、0フレーム目は複数回呼ばれ、回数が不定なのでスキップする
    if th19.round().unwrap().frame < 1 {
        let input_devices = th19.input_devices_mut();
        input_devices
            .p1_input_mut()
            .set_current(InputValue::empty());
        input_devices
            .p2_input_mut()
            .set_current(InputValue::empty());
    } else {
        let input_devices = th19.input_devices_mut();
        let delay = if session.host() {
            inputed_number(input_devices)
        } else {
            None
        };
        let (p1, p2) = session
            .enqueue_input_and_dequeue(input_devices.p1_input().current().bits() as u16, delay)?;
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());
    }
    Ok(())
}

pub fn on_round_over(session: &mut Session, th19: &mut Th19) -> Result<(), RecvError> {
    if session.host() {
        let init = session.init_round(Some(RoundInitial {
            seed1: th19.rand_seed1().unwrap(),
            seed2: th19.rand_seed2().unwrap(),
            seed3: th19.rand_seed3().unwrap(),
            seed4: th19.rand_seed4().unwrap(),
            seed5: th19.rand_seed5().unwrap(),
            seed6: th19.rand_seed6().unwrap(),
            seed7: th19.rand_seed7().unwrap(),
            seed8: th19.rand_seed8().unwrap(),
        }))?;
        assert!(init.is_none());
    } else {
        let init = session.init_round(None)?.unwrap();
        th19.set_rand_seed1(init.seed1).unwrap();
        th19.set_rand_seed2(init.seed2).unwrap();
        th19.set_rand_seed3(init.seed3).unwrap();
        th19.set_rand_seed4(init.seed4).unwrap();
        th19.set_rand_seed5(init.seed5).unwrap();
        th19.set_rand_seed6(init.seed6).unwrap();
        th19.set_rand_seed7(init.seed7).unwrap();
        th19.set_rand_seed8(init.seed8).unwrap();
    }
    Ok(())
}
