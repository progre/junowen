use junowen_lib::{
    th19_helpers::{
        move_to_local_versus_difficulty_select, move_to_title, resolve_keyboard_full_conflict,
    },
    PlayerMatchup, Th19,
};

pub fn on_input_menu(th19: &mut Th19, prepare_state: u8) {
    th19.set_no_wait(true);
    let Some(menu) = th19.app_mut().main_loop_tasks.find_menu_mut() else {
        return;
    };
    match prepare_state {
        0 => {
            move_to_title(th19, menu);
        }
        1 => {
            resolve_keyboard_full_conflict(th19, menu);
        }
        2 => {
            move_to_local_versus_difficulty_select(th19, menu, PlayerMatchup::HumanVsHuman);
        }
        _ => unreachable!(),
    }
}
