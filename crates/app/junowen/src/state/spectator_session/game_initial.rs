use junowen_lib::Th19;
use tracing::warn;

use crate::session::{RoundInitial, spectator::GameInitial};

pub fn set_rand_seeds(th19: &mut Th19, round_initial: &RoundInitial) {
    th19.set_rand_seed1(round_initial.seed1).unwrap();
    th19.set_rand_seed2(round_initial.seed2).unwrap();
    th19.set_rand_seed3(round_initial.seed3).unwrap();
    th19.set_rand_seed4(round_initial.seed4).unwrap();
}

/// 観戦者の試合開始時の状態がホストと一致しているか確認する
pub fn verify_game_initial(th19: &Th19, init: &GameInitial) -> bool {
    let selection = th19.selection();
    let actual = (
        selection.difficulty as u8,
        selection.p1().character as u8,
        selection.p1().card as u8,
        selection.p2().character as u8,
        selection.p2().card as u8,
    );
    let expected = (
        init.difficulty(),
        init.p1().character(),
        init.p1().card(),
        init.p2().character(),
        init.p2().card(),
    );
    if actual != expected {
        warn!(
            "game initial mismatch. expected={:?}, actual={:?}",
            expected, actual
        );
        return false;
    }
    true
}
