use anyhow::Result;
use junowen_lib::{Input, Th19};
use tracing::debug;

use crate::session::{RandomNumberInitial, Session};

pub fn on_input_players(session: &mut Session, th19: &mut Th19) -> Result<()> {
    // -1フレーム目、0フレーム目は複数回呼ばれ、回数が不定なのでスキップする
    if th19.game().unwrap().frame >= 1 {
        session.enqueue_input(th19.input().p1_input().0 as u8);
        let (p1, p2) = session.dequeue_inputs()?;
        let input = th19.input_mut();
        input.set_p1_input(Input(p1 as u32));
        input.set_p2_input(Input(p2 as u32));
    }
    Ok(())
}

pub fn on_round_over(session: &mut Session, th19: &mut Th19) {
    if session.host() {
        session.send_init_random_number(RandomNumberInitial {
            seed1: th19.rand_seed1().unwrap(),
            seed2: th19.rand_seed2().unwrap(),
            seed3: th19.rand_seed3().unwrap(),
            seed4: th19.rand_seed4().unwrap(),
            seed5: th19.rand_seed5().unwrap(),
            seed6: th19.rand_seed6().unwrap(),
            seed7: th19.rand_seed7().unwrap(),
            seed8: th19.rand_seed8().unwrap(),
        });
    } else {
        let (init, delay_remainings) = session.recv_init_round().unwrap();
        debug!("delay_remainings: {}", delay_remainings);
        th19.set_rand_seed1(init.seed1).unwrap();
        th19.set_rand_seed2(init.seed2).unwrap();
        th19.set_rand_seed3(init.seed3).unwrap();
        th19.set_rand_seed4(init.seed4).unwrap();
        th19.set_rand_seed5(init.seed5).unwrap();
        th19.set_rand_seed6(init.seed6).unwrap();
        th19.set_rand_seed7(init.seed7).unwrap();
        th19.set_rand_seed8(init.seed8).unwrap();
    }
}
