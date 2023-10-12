use std::sync::mpsc::RecvError;

use anyhow::Result;
use junowen_lib::{th19_helpers::reset_cursors, Input, Menu, ScreenId, Th19};

use crate::{
    helper::inputed_number,
    session::{MatchInitial, RoundInitial, Session},
};

pub fn on_input_players(
    first_time: bool,
    session: &mut Session,
    menu: &Menu,
    th19: &mut Th19,
    match_initial: &mut Option<MatchInitial>,
) -> Result<(), RecvError> {
    if first_time {
        th19.set_no_wait(false);
        reset_cursors(th19);

        if session.host() {
            if match_initial.is_none() {
                let init = MatchInitial {
                    game_settings: th19.game_settings_in_menu().unwrap(),
                };
                let (remote_player_name, opt) = session.init_match(
                    th19.player_name().player_name().to_string(),
                    Some(init.clone()),
                )?;
                session.set_remote_player_name(remote_player_name);
                debug_assert!(opt.is_none());
                *match_initial = Some(init);
            }
            let opt = session.init_round(Some(RoundInitial {
                seed1: th19.rand_seed1().unwrap(),
                seed2: th19.rand_seed2().unwrap(),
                seed3: th19.rand_seed3().unwrap(),
                seed4: th19.rand_seed4().unwrap(),
                seed5: th19.rand_seed5().unwrap(),
                seed6: th19.rand_seed6().unwrap(),
                seed7: th19.rand_seed7().unwrap(),
                seed8: th19.rand_seed8().unwrap(),
            }))?;
            debug_assert!(opt.is_none());
        } else {
            if match_initial.is_none() {
                let (remote_player_name, opt) =
                    session.init_match(th19.player_name().player_name().to_string(), None)?;
                session.set_remote_player_name(remote_player_name);
                debug_assert!(opt.is_some());
                *match_initial = opt;
            }
            let init = session.init_round(None)?.unwrap();
            th19.set_rand_seed1(init.seed1).unwrap();
            th19.set_rand_seed2(init.seed2).unwrap();
            th19.set_rand_seed3(init.seed3).unwrap();
            th19.set_rand_seed4(init.seed4).unwrap();
            th19.set_rand_seed5(init.seed5).unwrap();
            th19.set_rand_seed6(init.seed6).unwrap();
            th19.set_rand_seed7(init.seed7).unwrap();
            th19.set_rand_seed8(init.seed8).unwrap();
        }
    }

    if menu.screen_id == ScreenId::DifficultySelect {
        return Ok(());
    }

    let input_devices = th19.input_devices();
    let delay = if session.host() {
        inputed_number(input_devices)
    } else {
        None
    };
    let (p1, p2) = session.enqueue_input_and_dequeue(input_devices.p1_input().0 as u16, delay)?;
    let input_devices = th19.input_devices_mut();
    input_devices.set_p1_input(Input(p1 as u32));
    input_devices.set_p2_input(Input(p2 as u32));

    Ok(())
}

pub fn on_input_menu(session: &mut Session, th19: &mut Th19) -> Result<(), RecvError> {
    let menu = th19
        .app_mut()
        .main_loop_tasks_mut()
        .find_menu_mut()
        .unwrap();
    if menu.screen_id != ScreenId::DifficultySelect {
        return Ok(());
    }

    let delay = if session.host() {
        inputed_number(th19.input_devices())
    } else {
        None
    };
    let (p1, p2) = session.enqueue_input_and_dequeue(th19.menu_input().0 as u16, delay)?;
    let input = if p1 != 0 { p1 } else { p2 };
    th19.set_menu_input(Input(input as u32));
    Ok(())
}

pub fn on_loaded_game_settings(match_initial: &MatchInitial, th19: &mut Th19) {
    th19.put_game_settings_in_game(&match_initial.game_settings)
        .unwrap();
}
