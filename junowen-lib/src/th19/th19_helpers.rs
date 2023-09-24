use crate::{BattleSettings, Difficulty, GameMode, Input, Menu, PlayerMatchup, ScreenId, Th19};

pub struct InitialBattleInformation<'a> {
    pub difficulty: Difficulty,
    pub player_matchup: PlayerMatchup,
    pub battle_settings: &'a BattleSettings,
    pub p1_character: u8,
    pub p1_card: u8,
    pub p2_character: u8,
    pub p2_card: u8,
}

pub fn shot_repeatedly(prev: Input) -> Input {
    if prev.0 == Input::SHOT {
        Input(Input::NULL)
    } else {
        Input(Input::SHOT)
    }
}

pub fn select_cursor(
    current_input: &mut Input,
    prev_input: Input,
    current_cursor: &mut u32,
    target: u32,
) {
    if *current_cursor != target {
        *current_cursor = target;
    }
    *current_input = shot_repeatedly(prev_input);
}

pub fn move_to_local_versus_difficulty_select(
    th19: &mut Th19,
    menu: &mut Menu,
    target_player_matchup: PlayerMatchup,
) -> bool {
    match (
        menu.screen_id,
        th19.game_mode().unwrap(),
        th19.player_matchup().unwrap(),
    ) {
        (ScreenId::Loading, _, _) => false,
        (ScreenId::Title, _, _) => {
            select_cursor(
                th19.menu_input_mut(),
                *th19.prev_menu_input(),
                &mut menu.cursor,
                1,
            );
            false
        }
        (ScreenId::PlayerMatchupSelect, _, _) => {
            let target = if target_player_matchup == PlayerMatchup::HumanVsCpu {
                1
            } else {
                0
            };
            select_cursor(
                th19.menu_input_mut(),
                *th19.prev_menu_input(),
                &mut menu.cursor,
                target,
            );
            false
        }
        (
            ScreenId::DifficultySelect,
            GameMode::Versus,
            PlayerMatchup::HumanVsHuman | PlayerMatchup::HumanVsCpu | PlayerMatchup::CpuVsCpu,
        ) => true,
        (ScreenId::CharacterSelect, GameMode::Versus, _) => false,
        (ScreenId::BattleLoading, GameMode::Versus, _) => false,
        _ => {
            eprintln!("unknown screen {}", menu.screen_id as u32);
            false
        }
    }
}
