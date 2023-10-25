use derive_new::new;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use junowen_lib::{th19_helpers::AutomaticInputs, PlayerMatchup, Th19};

use crate::session::BattleSession;

fn to_automatic_inputs(prepare_state: u8) -> AutomaticInputs {
    match prepare_state {
        0 => AutomaticInputs::TransitionToTitle,
        1 => AutomaticInputs::ResolveKeyboardFullConflict,
        2 => AutomaticInputs::TransitionToLocalVersusDifficultySelect(PlayerMatchup::HumanVsHuman),
        _ => unreachable!(),
    }
}

#[derive(new, CopyGetters, Getters, MutGetters, Setters)]
pub struct Prepare {
    #[getset(get = "pub", get_mut = "pub")]
    battle_session: BattleSession,
    /// 0: back to title, 1: resolve controller, 2: forward to difficulty select
    #[getset(get_copy = "pub", set = "pub")]
    state: u8,
}

impl Prepare {
    pub fn inner_battle_session(self) -> BattleSession {
        self.battle_session
    }

    pub fn update_th19_on_input_players(&self, th19: &mut Th19) {
        th19.set_no_wait(true);
        to_automatic_inputs(self.state).on_input_players(th19);
    }

    pub fn update_th19_on_input_menu(&self, th19: &mut Th19) {
        let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
            return;
        };
        let no_wait = to_automatic_inputs(self.state).on_input_menu(th19, menu);
        th19.set_no_wait(no_wait);
    }
}
