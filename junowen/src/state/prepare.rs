use junowen_lib::{th19_helpers::AutomaticInputs, PlayerMatchup, Th19};

fn to_automatic_inputs(prepare_state: u8) -> AutomaticInputs {
    match prepare_state {
        0 => AutomaticInputs::TransitionToTitle,
        1 => AutomaticInputs::ResolveKeyboardFullConflict,
        2 => AutomaticInputs::TransitionToLocalVersusDifficultySelect(PlayerMatchup::HumanVsHuman),
        _ => unreachable!(),
    }
}

pub fn on_input_players(th19: &mut Th19, prepare_state: u8) {
    th19.set_no_wait(true);
    to_automatic_inputs(prepare_state).on_input_players(th19);
}

pub fn on_input_menu(th19: &mut Th19, prepare_state: u8) {
    let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
        return;
    };
    let no_wait = to_automatic_inputs(prepare_state).on_input_menu(th19, menu);
    th19.set_no_wait(no_wait);
}
