use std::sync::mpsc::RecvError;

use anyhow::Result;
use derive_new::new;
use getset::{Getters, MutGetters};
use junowen_lib::{th19_helpers::reset_cursors, InputValue, Menu, ScreenId, Th19};
use tracing::trace;

use crate::session::spectator::{self, SpectatorSession};

#[derive(new, Getters, MutGetters)]
pub struct SpectatorSelect {
    #[getset(get = "pub", get_mut = "pub")]
    session: SpectatorSession,
    #[new(value = "0")]
    initializing_state: u8,
}

impl SpectatorSelect {
    pub fn inner_session(self) -> SpectatorSession {
        self.session
    }

    pub fn update_th19_on_input_players(
        &mut self,
        menu: &Menu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if self.initializing_state == 0 {
            if self.session.spectator_initial().is_none() {
                self.initializing_state = 1;
                reset_cursors(th19);
                self.session.recv_init_spectator()?;
            } else {
                self.initializing_state = 2;
            }
            let round_initial = self.session.dequeue_init_round()?;
            th19.set_rand_seed1(round_initial.seed1).unwrap();
            th19.set_rand_seed2(round_initial.seed2).unwrap();
            th19.set_rand_seed3(round_initial.seed3).unwrap();
            th19.set_rand_seed4(round_initial.seed4).unwrap();
        }
        if menu.screen_id == ScreenId::DifficultySelect {
            return Ok(());
        }
        if self.initializing_state == 1 {
            let init = self.session.spectator_initial().unwrap();
            match init.initial_state().screen() {
                spectator::Screen::DifficultySelect => {
                    return Ok(());
                }
                spectator::Screen::CharacterSelect => unimplemented!(),
                spectator::Screen::Game => unimplemented!(),
            }
        }

        let (p1, p2) = self.session.dequeue_inputs()?;
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
        menu: &mut Menu,
        th19: &mut Th19,
    ) -> Result<(), RecvError> {
        if menu.screen_id != ScreenId::DifficultySelect {
            return Ok(());
        }
        if self.initializing_state == 1 {
            let init = self.session.spectator_initial().unwrap();
            trace!("spectator_initial: {:?}", init);
            let initial_state = init.initial_state();
            match initial_state.screen() {
                spectator::Screen::DifficultySelect => {
                    if menu.cursor != initial_state.difficulty() as u32 {
                        menu.cursor = initial_state.difficulty() as u32;
                        th19.menu_input_mut().set_current(InputValue::empty());
                        return Ok(());
                    }
                    let selection = th19.selection_mut();
                    selection.p1_mut().character = initial_state.p1_character() as u32;
                    selection.p2_mut().character = initial_state.p2_character() as u32;
                    th19.set_no_wait(false);
                    self.initializing_state = 2;
                }
                spectator::Screen::CharacterSelect => unimplemented!(),
                spectator::Screen::Game => unimplemented!(),
            }
        }

        let (p1, p2) = self.session.dequeue_inputs()?;
        let input = if p1 != 0 { p1 } else { p2 };
        th19.menu_input_mut()
            .set_current((input as u32).try_into().unwrap());
        Ok(())
    }
}
