use junowen_lib::{move_to_local_versus_difficulty_select, PlayerMatchup, Th19};

pub fn on_input_menu(th19: &mut Th19) {
    let Some(menu) = th19.app().main_loop_tasks.find_menu_mut() else {
        return;
    };

    move_to_local_versus_difficulty_select(th19, menu, PlayerMatchup::HumanVsHuman);
}
