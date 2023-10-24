use junowen_lib::{th19_helpers::AutomaticInputs, Menu, PlayerMatchup, ScreenId, Th19};

use super::State;

pub fn update_state(state: &mut State, prepare_state: u8) -> Option<(bool, Option<&'static Menu>)> {
    let Some(menu) = state.th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
        return Some((false, None));
    };
    match prepare_state {
        0 => {
            if menu.screen_id != ScreenId::Title {
                return Some((false, Some(menu)));
            }
            state.junowen_state.change_to_prepare(
                if state.th19.input_devices().is_conflict_input_device() {
                    1
                } else {
                    2
                },
            );
            Some((true, Some(menu)))
        }
        1 => {
            if state.th19.input_devices().is_conflict_input_device() {
                return Some((false, Some(menu)));
            }
            state.junowen_state.change_to_prepare(0);
            Some((true, Some(menu)))
        }
        2 => {
            if menu.screen_id != ScreenId::DifficultySelect {
                return Some((false, Some(menu)));
            }
            state.junowen_state.change_to_select();
            Some((true, Some(menu)))
        }
        _ => unreachable!(),
    }
}

fn to_automatic_inputs(prepare_state: u8) -> AutomaticInputs {
    match prepare_state {
        0 => AutomaticInputs::TransitionToTitle,
        1 => AutomaticInputs::ResolveKeyboardFullConflict,
        2 => AutomaticInputs::TransitionToLocalVersusDifficultySelect(PlayerMatchup::HumanVsHuman),
        _ => unreachable!(),
    }
}

pub fn update_th19_on_input_players(th19: &mut Th19, prepare_state: u8) {
    th19.set_no_wait(true);
    to_automatic_inputs(prepare_state).on_input_players(th19);
}

pub fn update_th19_on_input_menu(th19: &mut Th19, prepare_state: u8) {
    let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
        return;
    };
    let no_wait = to_automatic_inputs(prepare_state).on_input_menu(th19, menu);
    th19.set_no_wait(no_wait);
}
