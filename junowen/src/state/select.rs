use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use getset::{Getters, MutGetters};
use junowen_lib::{th19_helpers::reset_cursors, Menu, ScreenId, Th19};

use crate::{
    helper::inputed_number,
    session::{BattleSession, MatchInitial, RoundInitial},
};

fn init_match(th19: &mut Th19, battle_session: &mut BattleSession) -> Result<(), RecvError> {
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
            let (remote_player_name, opt) =
                battle_session.init_match(th19.player_name().player_name().to_string(), None)?;
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
    Ok(())
}

#[derive(new, Getters, MutGetters)]
pub struct Select {
    #[getset(get = "pub", get_mut = "pub")]
    battle_session: BattleSession,
    #[new(value = "true")]
    first_time: bool,
}

impl Select {
    pub fn inner_battle_session(self) -> BattleSession {
        self.battle_session
    }

    pub fn update_th19_on_input_players(
        &mut self,
        menu: &Menu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if self.first_time {
            self.first_time = false;
            init_match(th19, &mut self.battle_session)?;
        }

        if menu.screen_id == ScreenId::DifficultySelect {
            return Ok(());
        }

        let input_devices = th19.input_devices_mut();
        let delay = if self.battle_session.host() {
            inputed_number(input_devices)
        } else {
            None
        };
        let (p1, p2) = self
            .battle_session
            .enqueue_input_and_dequeue(input_devices.p1_input().current().bits() as u16, delay)?;
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());

        Ok(())
    }

    pub fn update_th19_on_input_menu(&mut self, th19: &mut Th19) -> Result<(), RecvError> {
        let menu = th19.app().main_loop_tasks().find_menu().unwrap();
        if menu.screen_id != ScreenId::DifficultySelect {
            return Ok(());
        }

        let delay = if self.battle_session.host() {
            inputed_number(th19.input_devices())
        } else {
            None
        };
        let menu_input = th19.menu_input_mut();
        let (p1, p2) = self
            .battle_session
            .enqueue_input_and_dequeue(menu_input.current().bits() as u16, delay)?;
        let input = if p1 != 0 { p1 } else { p2 };
        menu_input.set_current((input as u32).try_into().unwrap());
        Ok(())
    }
}
