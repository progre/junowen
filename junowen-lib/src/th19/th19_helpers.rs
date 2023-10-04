use tracing::warn;

use crate::{GameMode, Input, Menu, PlayerMatchup, ScreenId, Th19};

pub fn reset_cursors(th19: &mut Th19) {
    th19.set_difficulty_cursor(1).unwrap();
    th19.p1_mut().character = 0;
    th19.p2_mut().character = 1;
    // NOTE: cards does'nt reset.
    //       it will reset in title screen, and online vs disconnected.
}

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

pub fn move_to_title(th19: &mut Th19, menu: &Menu) {
    *th19.menu_input_mut() = match (
        menu.screen_id,
        th19.game_mode().unwrap(),
        th19.player_matchup().unwrap(),
    ) {
        (ScreenId::TitleLoading, _, _) => Input(0),
        (ScreenId::Title, _, _) => Input(0),
        (
            ScreenId::PlayerMatchupSelect | ScreenId::DifficultySelect | ScreenId::CharacterSelect,
            _,
            _,
        ) => charge_repeatedly(*th19.prev_menu_input()),
        (ScreenId::GameLoading, _, _) => Input(0),
        _ => {
            warn!("unsupported screen {}", menu.screen_id as u32);
            Input(0)
        }
    }
}

pub fn move_to_local_versus_difficulty_select(
    th19: &mut Th19,
    menu: &mut Menu,
    target_player_matchup: PlayerMatchup,
) {
    *th19.menu_input_mut() = match (
        menu.screen_id,
        th19.game_mode().unwrap(),
        th19.player_matchup().unwrap(),
    ) {
        (ScreenId::TitleLoading, _, _) => Input(0),
        (ScreenId::Title, _, _) => select_cursor(*th19.prev_menu_input(), &mut menu.cursor, 1),
        (ScreenId::PlayerMatchupSelect, _, _) => {
            let target = if target_player_matchup == PlayerMatchup::HumanVsCpu {
                1
            } else {
                0
            };
            select_cursor(*th19.prev_menu_input(), &mut menu.cursor, target)
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
    }
}
