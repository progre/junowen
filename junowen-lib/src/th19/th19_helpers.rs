use tracing::{trace, warn};

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

fn escape_repeatedly(prev: Input) -> Input {
    if prev.0 == Input::START as u32 {
        Input(Input::NULL as u32)
    } else {
        Input(Input::START as u32)
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
        .main_loop_tasks()
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

pub enum AutomaticInputs {
    TransitionToTitle,
    ResolveKeyboardFullConflict,
    TransitionToLocalVersusDifficultySelect(PlayerMatchup),
}

impl AutomaticInputs {
    pub fn on_input_players(&self, th19: &mut Th19) {
        match self {
            Self::TransitionToTitle => transfer_to_title_on_input_players(th19),
            _ => {
                let (p1, p2) = (Input::NULL.into(), Input::NULL.into());
                th19.input_devices_mut().set_p1_input(p1);
                th19.input_devices_mut().set_p2_input(p2);
            }
        }
    }

    pub fn on_input_menu(&self, th19: &mut Th19, menu: &mut Menu) -> bool {
        match self {
            Self::TransitionToTitle => transfer_to_title_on_input_menu(th19, menu),
            Self::ResolveKeyboardFullConflict => resolve_keyboard_full_conflict(th19, menu),
            Self::TransitionToLocalVersusDifficultySelect(target_player_matchup) => {
                transfer_to_local_versus_difficulty_select(th19, menu, *target_player_matchup)
            }
        }
    }
}

fn transfer_to_title_on_input_players(th19: &mut Th19) {
    let input_devices = th19.input_devices_mut();
    let (p1, p2) = if let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() {
        match menu.screen_id {
            ScreenId::CharacterSelect => (
                escape_repeatedly(input_devices.p1_prev_input()),
                escape_repeatedly(input_devices.p2_prev_input()),
            ),
            ScreenId::Archievements => (
                Input::SHOT.into(), // skip ending
                Input::NULL.into(),
            ),
            ScreenId::Option => return,
            _ => (
                escape_repeatedly(th19.prev_menu_input()),
                Input::NULL.into(),
            ),
        }
    } else if let Some(game) = th19.app_mut().main_loop_tasks_mut().find_game_mut() {
        if game.pause() == 0 {
            (
                escape_repeatedly(input_devices.p1_prev_input()),
                escape_repeatedly(input_devices.p2_prev_input()),
            )
        } else if game.depth() == 0 {
            game.set_cursor(1);
            (
                shot_repeatedly(input_devices.p1_prev_input()),
                shot_repeatedly(input_devices.p2_prev_input()),
            )
        } else {
            game.set_cursor(0);
            (
                shot_repeatedly(input_devices.p1_prev_input()),
                shot_repeatedly(input_devices.p2_prev_input()),
            )
        }
    } else {
        (Input::NULL.into(), Input::NULL.into())
    };
    input_devices.set_p1_input(p1);
    input_devices.set_p2_input(p2);
}

fn transfer_to_title_on_input_menu(th19: &mut Th19, menu: &Menu) -> bool {
    trace!("menu.screen_id: {:x?}", menu.screen_id);
    let menu_input = match menu.screen_id {
        ScreenId::TitleLoading => return false,
        ScreenId::Title => Input(0),
        ScreenId::ControllerSelect => 'a: {
            let Some(ctrler_select) = th19
                .app_mut()
                .main_loop_tasks_mut()
                .find_controller_select_mut()
            else {
                break 'a Input(Input::NULL as u32);
            };
            if ctrler_select.depth == 1 {
                return false;
            }
            select_cursor(th19.prev_menu_input(), &mut ctrler_select.cursor, 3)
        }
        ScreenId::Option => 'a: {
            if th19
                .app_mut()
                .main_loop_tasks_mut()
                .find_controller_select_mut()
                .is_none()
            {
                break 'a escape_repeatedly(th19.prev_menu_input());
            }
            // NOTE: Can't determine whether it is in key config or not,
            //       so control is not possible.
            return false;
        }
        _ => escape_repeatedly(th19.prev_menu_input()),
    };
    th19.set_menu_input(menu_input);
    true
}

fn resolve_keyboard_full_conflict(th19: &mut Th19, menu: &mut Menu) -> bool {
    if !th19.input_devices().is_conflict_keyboard_full() {
        return true;
    }
    let menu_input = match (
        menu.screen_id,
        th19.selection().game_mode,
        th19.selection().player_matchup,
    ) {
        (ScreenId::Title, _, _) => select_cursor(th19.prev_menu_input(), &mut menu.cursor, 1),
        (ScreenId::PlayerMatchupSelect, _, _) => {
            select_cursor(th19.prev_menu_input(), &mut menu.cursor, 4)
        }
        (ScreenId::ControllerSelect, _, _) => {
            if let Some(ctrler_select) = th19
                .app_mut()
                .main_loop_tasks_mut()
                .find_controller_select_mut()
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
        _ => escape_repeatedly(th19.prev_menu_input()),
    };
    th19.set_menu_input(menu_input);
    true
}

fn transfer_to_local_versus_difficulty_select(
    th19: &mut Th19,
    menu: &mut Menu,
    target_player_matchup: PlayerMatchup,
) -> bool {
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
    );
    true
}
