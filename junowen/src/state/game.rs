use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::{Input, Th19};

use crate::{
    helper::inputed_number,
    session::{RoundInitial, Session},
};

pub fn on_input_players(session: &mut Session, th19: &mut Th19) -> Result<(), RecvError> {
    // -1フレーム目、0フレーム目は複数回呼ばれ、回数が不定なのでスキップする
    if th19.game().unwrap().frame < 1 {
        let input_devices = th19.input_devices_mut();
        input_devices.set_p1_input(Input(0));
        input_devices.set_p2_input(Input(0));
    } else {
        let input_devices = th19.input_devices();
        let delay = if session.host() {
            inputed_number(input_devices)
        } else {
            None
        };
        let (p1, p2) =
            session.enqueue_input_and_dequeue(input_devices.p1_input().0 as u16, delay)?;
        let input_devices = th19.input_devices_mut();
        input_devices.set_p1_input(Input(p1 as u32));
        input_devices.set_p2_input(Input(p2 as u32));
    }
    Ok(())
}

pub fn on_round_over(session: &mut Session, th19: &mut Th19) -> Result<(), RecvError> {
    if session.host() {
        let init = session.init_round(Some(RoundInitial {
            seed1: th19.rand_seed1().unwrap(),
            seed2: th19.rand_seed2().unwrap(),
        }))?;
        assert!(init.is_none());
    } else {
        let init = session.init_round(None)?.unwrap();
        th19.set_rand_seed1(init.seed1).unwrap();
        th19.set_rand_seed2(init.seed2).unwrap();
    }
    Ok(())
}
