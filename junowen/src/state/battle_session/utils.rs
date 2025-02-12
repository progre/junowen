use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::Th19;

use crate::session::{battle::BattleSession, RoundInitial};

use super::spectator_host::SpectatorHostState;

pub fn init_round(
    th19: &mut Th19,
    battle_session: &mut BattleSession,
    spectator_host_state: &mut SpectatorHostState,
) -> Result<(), RecvError> {
    if battle_session.host() {
        let opt = battle_session.init_round(Some(RoundInitial {
            seed1: th19.rand_seed1().unwrap(),
            seed2: th19.rand_seed2().unwrap(),
            seed3: th19.rand_seed3().unwrap(),
            seed4: th19.rand_seed4().unwrap(),
        }))?;
        debug_assert!(opt.is_none());
    } else {
        let init = battle_session.init_round(None)?.unwrap();
        th19.set_rand_seed1(init.seed1).unwrap();
        th19.set_rand_seed2(init.seed2).unwrap();
        th19.set_rand_seed3(init.seed3).unwrap();
        th19.set_rand_seed4(init.seed4).unwrap();
    }
    spectator_host_state.send_init_round_if_connected(th19);
    Ok(())
}
