use tracing::{trace, warn};

use crate::{GameMode, InputFlags, InputValue, PlayerMatchup, Th19};

use super::app::{MainMenu, ScreenId};

pub fn shot_repeatedly(prev: InputValue) -> InputValue {
    if prev == InputFlags::SHOT.into() {
        InputValue::empty()
    } else {
        InputFlags::SHOT.into()
    }
}

fn escape_repeatedly(prev: InputValue) -> InputValue {
    if prev == InputFlags::PAUSE.into() {
        InputValue::empty()
    } else {
        InputFlags::PAUSE.into()
    }
}

pub fn select_cursor(prev_input: InputValue, current_cursor: &mut u32, target: u32) -> InputValue {
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
                let (p1, p2) = (InputValue::empty(), InputValue::empty());
                let input_devices = th19.input_devices_mut();
                input_devices.p1_input_mut().set_current(p1);
                input_devices.p2_input_mut().set_current(p2);
            }
        }
    }

    pub fn on_input_menu(&self, th19: &mut Th19, main_menu: &mut MainMenu) -> bool {
        match self {
            Self::TransitionToTitle => transfer_to_title_on_input_menu(th19, main_menu),
            Self::ResolveKeyboardFullConflict => resolve_input_device_conflict(th19, main_menu),
            Self::TransitionToLocalVersusDifficultySelect(target_player_matchup) => {
                transfer_to_local_versus_difficulty_select(th19, main_menu, *target_player_matchup)
            }
        }
    }
}

fn transfer_to_title_on_input_players(th19: &mut Th19) {
    let input_devices = th19.input_devices_mut();
    let main_menu = th19.app_mut().main_loop_tasks_mut().find_main_menu_mut();
    let (p1, p2) = if let Some(main_menu) = main_menu {
        match main_menu.screen_id() {
            ScreenId::CharacterSelect => (
                escape_repeatedly(input_devices.p1_input().prev()),
                escape_repeatedly(input_devices.p2_input().prev()),
            ),
            ScreenId::Archievements => (
                InputFlags::SHOT.into(), // skip ending
                InputValue::empty(),
            ),
            ScreenId::Option => return,
            _ => (
                escape_repeatedly(th19.menu_input().prev()),
                InputValue::empty(),
            ),
        }
    } else if let Some(game) = th19.app_mut().main_loop_tasks_mut().find_game_mut() {
        if game.pause() == 0 {
            (
                escape_repeatedly(input_devices.p1_input().prev()),
                escape_repeatedly(input_devices.p2_input().prev()),
            )
        } else if game.depth() == 0 {
            game.set_cursor(1);
            (
                shot_repeatedly(input_devices.p1_input().prev()),
                shot_repeatedly(input_devices.p2_input().prev()),
            )
        } else {
            game.set_cursor(0);
            (
                shot_repeatedly(input_devices.p1_input().prev()),
                shot_repeatedly(input_devices.p2_input().prev()),
            )
        }
    } else {
        (InputValue::empty(), InputValue::empty())
    };
    input_devices.p1_input_mut().set_current(p1);
    input_devices.p2_input_mut().set_current(p2);
}

fn transfer_to_title_on_input_menu(th19: &mut Th19, main_menu: &MainMenu) -> bool {
    trace!("menu.screen_id: {:x?}", main_menu.screen_id());
    let menu_input = match main_menu.screen_id() {
        ScreenId::TitleLoading => return false,
        ScreenId::Title => InputValue::empty(),
        ScreenId::ControllerSelect => 'a: {
            let Some(ctrler_select) = th19
                .app_mut()
                .main_loop_tasks_mut()
                .find_controller_select_mut()
            else {
                break 'a InputValue::empty();
            };
            if ctrler_select.depth == 1 {
                return false;
            }
            select_cursor(th19.menu_input().prev(), &mut ctrler_select.cursor, 3)
        }
        ScreenId::Option => 'a: {
            if th19
                .app_mut()
                .main_loop_tasks_mut()
                .find_controller_select_mut()
                .is_none()
            {
                break 'a escape_repeatedly(th19.menu_input().prev());
            }
            // NOTE: Can't determine whether it is in key config or not,
            //       so control is not possible.
            return false;
        }
        _ => escape_repeatedly(th19.menu_input().prev()),
    };
    th19.menu_input_mut().set_current(menu_input);
    true
}

fn resolve_input_device_conflict(th19: &mut Th19, main_menu: &mut MainMenu) -> bool {
    if !th19.input_devices().is_conflict_input_device() {
        return true;
    }
    let screen_id = main_menu.screen_id();
    let menu = main_menu;
    let menu_input = match (
        screen_id,
        th19.selection().game_mode,
        th19.selection().player_matchup,
    ) {
        (ScreenId::Title, _, _) => select_cursor(th19.menu_input().prev(), menu.cursor_mut(), 1),
        (ScreenId::PlayerMatchupSelect, _, _) => {
            select_cursor(th19.menu_input().prev(), menu.cursor_mut(), 4)
        }
        (ScreenId::ControllerSelect, _, _) => {
            if let Some(ctrler_select) = th19
                .app_mut()
                .main_loop_tasks_mut()
                .find_controller_select_mut()
            {
                ctrler_select.cursor = 1;
                if th19.menu_input().prev() == InputFlags::LEFT.into() {
                    InputValue::empty()
                } else {
                    InputFlags::LEFT.into()
                }
            } else {
                InputValue::empty()
            }
        }
        _ => escape_repeatedly(th19.menu_input().prev()),
    };
    th19.menu_input_mut().set_current(menu_input);
    true
}

fn transfer_to_local_versus_difficulty_select(
    th19: &mut Th19,
    main_menu: &mut MainMenu,
    target_player_matchup: PlayerMatchup,
) -> bool {
    let screen_id = main_menu.screen_id();
    let menu = main_menu;
    th19.menu_input_mut().set_current(
        match (
            screen_id,
            th19.selection().game_mode,
            th19.selection().player_matchup,
        ) {
            (ScreenId::TitleLoading, _, _) => InputValue::empty(),
            (ScreenId::Title, _, _) => {
                select_cursor(th19.menu_input_mut().prev(), menu.cursor_mut(), 1)
            }
            (ScreenId::PlayerMatchupSelect, _, _) => {
                let target = if target_player_matchup == PlayerMatchup::HumanVsCpu {
                    1
                } else {
                    0
                };
                select_cursor(th19.menu_input_mut().prev(), menu.cursor_mut(), target)
            }
            (
                ScreenId::DifficultySelect,
                GameMode::Versus,
                PlayerMatchup::HumanVsHuman | PlayerMatchup::HumanVsCpu | PlayerMatchup::CpuVsCpu,
            ) => InputValue::empty(),
            _ => {
                warn!("unsupported screen {}", screen_id as u32);
                InputValue::empty()
            }
        },
    );
    true
}
