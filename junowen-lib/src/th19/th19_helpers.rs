use crate::{
    BattleSettings, DevicesInput, Difficulty, GameMode, Input, Menu, PlayerMatchup, ScreenId, Th19,
};

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

pub fn select_cursor(input: &mut DevicesInput, current: &mut u32, target: u32) {
    if *current != target {
        *current = target;
    }
    input.set_p1_input(shot_repeatedly(input.p1_prev_input()));
    input.set_p2_input(Input(Input::NULL));
}

pub fn move_to_local_versus_difficulty_select(
    th19: &mut Th19,
    menu: &mut Menu,
    inits: &InitialBattleInformation,
) -> bool {
    match (
        menu.screen_id,
        th19.game_mode().unwrap(),
        th19.player_matchup().unwrap(),
    ) {
        (ScreenId::Loading, _, _) => {
            let input = th19.input_mut();
            input.set_p1_input(Input(Input::NULL));
            input.set_p2_input(Input(Input::NULL));
            false
        }
        (ScreenId::Title, _, _) => {
            select_cursor(th19.input_mut(), &mut menu.cursor, 1);
            false
        }
        (ScreenId::PlayerMatchupSelect, _, _) => {
            let target = if inits.player_matchup == PlayerMatchup::HumanVsCpu {
                1
            } else {
                0
            };
            select_cursor(th19.input_mut(), &mut menu.cursor, target);
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
