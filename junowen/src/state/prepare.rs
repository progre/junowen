use derive_new::new;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use junowen_lib::{th19_helpers::AutomaticInputs, MainMenu, PlayerMatchup, ScreenId, Th19};

fn to_automatic_inputs(prepare_state: u8) -> AutomaticInputs {
    match prepare_state {
        0 => AutomaticInputs::TransitionToTitle,
        1 => AutomaticInputs::ResolveKeyboardFullConflict,
        2 => AutomaticInputs::TransitionToLocalVersusDifficultySelect(PlayerMatchup::HumanVsHuman),
        _ => unreachable!(),
    }
}

#[derive(new, CopyGetters, Getters, MutGetters, Setters)]
pub struct Prepare<T> {
    #[getset(get = "pub", get_mut = "pub")]
    session: T,
    /// 0: back to title, 1: resolve controller, 2: forward to difficulty select
    #[new(value = "0")]
    state: u8,
}

impl<T> Prepare<T> {
    pub fn inner_session(self) -> T {
        self.session
    }

    pub fn update_th19_on_input_players(&self, th19: &mut Th19) {
        th19.set_no_wait(true);
        to_automatic_inputs(self.state).on_input_players(th19);
    }

    pub fn update_th19_on_input_menu(&self, th19: &mut Th19) {
        let Some(main_menu) = th19.app_mut().main_loop_tasks_mut().find_main_menu_mut() else {
            return;
        };
        let no_wait = to_automatic_inputs(self.state).on_input_menu(th19, main_menu);
        th19.set_no_wait(no_wait);
    }

    pub fn update_state(&mut self, main_menu: &MainMenu, th19: &Th19) -> bool {
        match self.state {
            0 => {
                if main_menu.screen_id() != ScreenId::Title {
                    return false;
                }
                let new_state = if th19.input_devices().is_conflict_input_device() {
                    1
                } else {
                    2
                };
                self.state = new_state;
                false
            }
            1 => {
                if th19.input_devices().is_conflict_input_device() {
                    return false;
                }
                self.state = 0;
                false
            }
            2 => {
                if main_menu.screen_id() != ScreenId::DifficultySelect {
                    return false;
                }
                true
            }
            _ => unreachable!(),
        }
    }
}
