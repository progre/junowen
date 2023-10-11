use tracing::warn;

use crate::{GameMode, PlayerMatchup, Th19};

use super::{
    app::{Menu, ScreenId},
    inputdevices::Input,
};

pub fn shot_repeatedly(prev: Input) -> Input {
    if prev.0 == Input::SHOT as u32 {
        Input(Input::NULL as u32)
    } else {
        Input(Input::SHOT as u32)
    }
}

fn charge_repeatedly(prev: Input) -> Input {
    if prev.0 == Input::CHARGE as u32 {
        Input(Input::NULL as u32)
    } else {
        Input(Input::CHARGE as u32)
    }
}

pub fn select_cursor(prev_input: Input, current_cursor: &mut u32, target: u32) -> Input {
    if *current_cursor != target {
        *current_cursor = target;
    }
    shot_repeatedly(prev_input)
}

// -----------------------------------------------------------------------------

pub fn is_network_mode(th19: &Th19) -> bool {
    if th19.selection().game_mode == GameMode::Story {
        return false;
    }
    // VS Mode 最初の階層では player_matchup がまだセットされないので、オンライン用メイン関数がセットされているかどうかで判断する
    th19.app()
        .main_loop_tasks
        .to_vec()
        .iter()
        .any(|item| item.id == 3 || item.id == 4)
}

pub fn reset_cursors(th19: &mut Th19) {
    th19.set_difficulty_cursor(1).unwrap();
    th19.selection_mut().p1_mut().character = 0;
    th19.selection_mut().p2_mut().character = 1;
    // NOTE: cards does'nt reset.
    //       it will reset in title screen, and online vs disconnected.
}

pub fn move_to_title(th19: &mut Th19, menu: &Menu) {
    th19.set_menu_input(
        match (
            menu.screen_id,
            th19.selection().game_mode,
            th19.selection().player_matchup,
        ) {
            (ScreenId::Title, _, _) => Input(0),
            (ScreenId::ControllerSelect, _, _) => {
                if let Some(ctrler_select) = th19.app().main_loop_tasks.find_controller_select_mut()
                {
                    select_cursor(th19.prev_menu_input(), &mut ctrler_select.cursor, 3)
                } else {
                    Input(Input::NULL as u32)
                }
            }
            _ => charge_repeatedly(th19.prev_menu_input()),
        },
    )
}

pub fn resolve_keyboard_full_conflict(th19: &mut Th19, menu: &mut Menu) -> bool {
    if !th19.input_devices().is_conflict_keyboard_full() {
        return true;
    }
    th19.set_menu_input(
        match (
            menu.screen_id,
            th19.selection().game_mode,
            th19.selection().player_matchup,
        ) {
            (ScreenId::Title, _, _) => select_cursor(th19.prev_menu_input(), &mut menu.cursor, 1),
            (ScreenId::PlayerMatchupSelect, _, _) => {
                select_cursor(th19.prev_menu_input(), &mut menu.cursor, 4)
            }
            (ScreenId::ControllerSelect, _, _) => {
                if let Some(ctrler_select) = th19.app().main_loop_tasks.find_controller_select_mut()
                {
                    ctrler_select.cursor = 1;
                    if th19.prev_menu_input().0 == Input::LEFT as u32 {
                        Input(Input::NULL as u32)
                    } else {
                        Input(Input::LEFT as u32)
                    }
                } else {
                    Input(Input::NULL as u32)
                }
            }
            _ => charge_repeatedly(th19.prev_menu_input()),
        },
    );
    false
}

pub fn move_to_local_versus_difficulty_select(
    th19: &mut Th19,
    menu: &mut Menu,
    target_player_matchup: PlayerMatchup,
) {
    th19.set_menu_input(
        match (
            menu.screen_id,
            th19.selection().game_mode,
            th19.selection().player_matchup,
        ) {
            (ScreenId::TitleLoading, _, _) => Input(0),
            (ScreenId::Title, _, _) => select_cursor(th19.prev_menu_input(), &mut menu.cursor, 1),
            (ScreenId::PlayerMatchupSelect, _, _) => {
                let target = if target_player_matchup == PlayerMatchup::HumanVsCpu {
                    1
                } else {
                    0
                };
                select_cursor(th19.prev_menu_input(), &mut menu.cursor, target)
            }
            (
                ScreenId::DifficultySelect,
                GameMode::Versus,
                PlayerMatchup::HumanVsHuman | PlayerMatchup::HumanVsCpu | PlayerMatchup::CpuVsCpu,
            ) => Input(0),
            _ => {
                warn!("unsupported screen {}", menu.screen_id as u32);
                Input(0)
            }
        },
    )
}
