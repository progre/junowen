use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use junowen_lib::{
    Th19,
    structs::app::{MainMenu, ScreenId},
};

use crate::session::spectator::SpectatorSession;

use super::set_rand_seeds;

#[derive(new)]
pub struct SpectatorSelect {
    /// 対戦後に戻ってきた場合は、最初のフレームでラウンドの初期化情報を受け取る
    recv_round_initial: bool,
}

impl SpectatorSelect {
    pub fn update_th19_on_input_players(
        &mut self,
        session: &mut SpectatorSession,
        main_menu: &MainMenu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if self.recv_round_initial {
            self.recv_round_initial = false;
            let round_initial = session.dequeue_init_round()?;
            set_rand_seeds(th19, &round_initial);
        }
        if main_menu.screen_id() == ScreenId::DifficultySelect {
            return Ok(());
        }
        if !th19.no_wait() {
            th19.set_no_wait(true);
        }

        let (p1, p2) = session.dequeue_inputs()?;
        let input_devices = th19.input_devices_mut();
        input_devices
            .p1_input_mut()
            .set_current((p1 as u32).try_into().unwrap());
        input_devices
            .p2_input_mut()
            .set_current((p2 as u32).try_into().unwrap());

        Ok(())
    }

    pub fn update_th19_on_input_menu(
        &mut self,
        session: &mut SpectatorSession,
        main_menu: &mut MainMenu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if main_menu.screen_id() != ScreenId::DifficultySelect {
            return Ok(());
        }
        let (p1, p2) = session.dequeue_inputs()?;
        let input = if p1 != 0 { p1 } else { p2 };
        th19.menu_input_mut()
            .set_current((input as u32).try_into().unwrap());
        Ok(())
    }
}
