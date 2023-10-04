use junowen_lib::{move_to_local_versus_difficulty_select, move_to_title, PlayerMatchup, Th19};

pub fn on_input_menu(th19: &mut Th19, passing_title: bool) {
    let Some(menu) = th19.app().main_loop_tasks.find_menu_mut() else {
        return;
    };
    if !passing_title {
        move_to_title(th19, menu);
    } else {
        move_to_local_versus_difficulty_select(th19, menu, PlayerMatchup::HumanVsHuman);
    }
}
