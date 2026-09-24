use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::{Th19, structs::input_devices::InputValue};

use crate::{
    helper::{inputed_number, pushed_f1},
    session::battle::BattleSession,
};

use super::{
    spectator_host::{SpectatorHostState, SpectatorMatchInfo},
    utils::init_round,
};

pub struct BattleGame;

impl BattleGame {
    pub fn update_th19(
        &mut self,
        session: &mut BattleSession,
        spectator_host_state: &mut SpectatorHostState,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
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
        let f1_pushed = pushed_f1(th19.input_devices());
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

        let match_info = SpectatorMatchInfo::from_battle_session(session, th19);
        spectator_host_state.update(f1_pushed, None, th19, match_info, p1, p2);

        Ok(())
    }

    pub fn on_round_over(
        &mut self,
        session: &mut BattleSession,
        spectator_host_state: &mut SpectatorHostState,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        init_round(th19, session, spectator_host_state)
    }
}
