use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::{Th19, structs::input_devices::InputValue};

use crate::session::spectator::SpectatorSession;

use super::set_rand_seeds;

/// 未処理の入力がこのフレーム数を超えている場合、早送りして追いつく
const CATCH_UP_THRESHOLD: usize = 30;

pub struct SpectatorGame;

impl SpectatorGame {
    pub fn update_th19(
        &mut self,
        session: &mut SpectatorSession,
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
        // 観戦者の試合開始はホストより遅れるため、溜まった入力を早送りしてホストに追いつく
        let no_wait = session.poll_buffered_len() > CATCH_UP_THRESHOLD;
        if th19.no_wait() != no_wait {
            th19.set_no_wait(no_wait);
        }
        let input_devices = th19.input_devices_mut();
        let (p1, p2) = session.dequeue_inputs()?;
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());
        Ok(())
    }

    pub fn on_round_over(
        &mut self,
        session: &mut SpectatorSession,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if let Some(init) = session.dequeue_init_round()? {
            set_rand_seeds(th19, &init);
        }
        Ok(())
    }
}
