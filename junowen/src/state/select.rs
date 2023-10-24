use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::{th19_helpers::reset_cursors, Menu, ScreenId, Th19};

use crate::{
    helper::inputed_number,
    session::{BattleSession, MatchInitial, RoundInitial},
};

use super::State;

pub fn update_state(state: &mut State) -> Option<(bool, Option<&'static Menu>)> {
    let menu = state
        .th19
        .app_mut()
        .main_loop_tasks_mut()
        .find_menu_mut()
        .unwrap();
    match menu.screen_id {
        ScreenId::GameLoading => {
            state.junowen_state.change_to_game_loading();
            Some((true, Some(menu)))
        }
        ScreenId::PlayerMatchupSelect => {
            state.end_session();
            None
        }
        _ => Some((false, Some(menu))),
    }
}

pub fn update_th19_on_input_players(
    first_time: bool,
    battle_session: &mut BattleSession,
    menu: &Menu,
    th19: &mut Th19,
) -> Result<(), RecvError> {
    if first_time {
        th19.set_no_wait(false);
        reset_cursors(th19);

        if battle_session.host() {
            if battle_session.match_initial().is_none() {
                let init = MatchInitial {
                    game_settings: th19.game_settings_in_menu().unwrap(),
                };
                let (remote_player_name, opt) = battle_session.init_match(
                    th19.player_name().player_name().to_string(),
                    Some(init.clone()),
                )?;
                battle_session.set_remote_player_name(remote_player_name);
                debug_assert!(opt.is_none());
                battle_session.set_match_initial(Some(init));
            }
            let opt = battle_session.init_round(Some(RoundInitial {
                seed1: th19.rand_seed1().unwrap(),
                seed2: th19.rand_seed2().unwrap(),
                seed3: th19.rand_seed3().unwrap(),
                seed4: th19.rand_seed4().unwrap(),
            }))?;
            debug_assert!(opt.is_none());
        } else {
            if battle_session.match_initial().is_none() {
                let (remote_player_name, opt) = battle_session
                    .init_match(th19.player_name().player_name().to_string(), None)?;
                battle_session.set_remote_player_name(remote_player_name);
                debug_assert!(opt.is_some());
                battle_session.set_match_initial(opt);
            }
            let init = battle_session.init_round(None)?.unwrap();
            th19.set_rand_seed1(init.seed1).unwrap();
            th19.set_rand_seed2(init.seed2).unwrap();
            th19.set_rand_seed3(init.seed3).unwrap();
            th19.set_rand_seed4(init.seed4).unwrap();
        }
    }

    if menu.screen_id == ScreenId::DifficultySelect {
        return Ok(());
    }

    let input_devices = th19.input_devices_mut();
    let delay = if battle_session.host() {
        inputed_number(input_devices)
    } else {
        None
    };
    let (p1, p2) = battle_session
        .enqueue_input_and_dequeue(input_devices.p1_input().current().bits() as u16, delay)?;
    input_devices
        .p1_input_mut()
        .set_current((p1 as u32).try_into().unwrap());
    input_devices
        .p2_input_mut()
        .set_current((p2 as u32).try_into().unwrap());

    Ok(())
}

pub fn update_th19_on_input_menu(
    battle_session: &mut BattleSession,
    th19: &mut Th19,
) -> Result<(), RecvError> {
    let menu = th19
        .app_mut()
        .main_loop_tasks_mut()
        .find_menu_mut()
        .unwrap();
    if menu.screen_id != ScreenId::DifficultySelect {
        return Ok(());
    }

    let delay = if battle_session.host() {
        inputed_number(th19.input_devices())
    } else {
        None
    };
    let menu_input = th19.menu_input_mut();
    let (p1, p2) =
        battle_session.enqueue_input_and_dequeue(menu_input.current().bits() as u16, delay)?;
    let input = if p1 != 0 { p1 } else { p2 };
    menu_input.set_current((input as u32).try_into().unwrap());
    Ok(())
}
