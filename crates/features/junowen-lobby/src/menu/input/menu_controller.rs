use super::MenuButtons;

pub enum MenuControllerUpdateDecideResult {
    None,
    Wait,
    Decide,
    Cancel,
}

pub enum MenuControllerInputResult {
    None,
    Cancel,
    Decide,
    Up,
    Down,
}

#[derive(Default)]
pub struct MenuController {
    repeat_up: u32,
    repeat_down: u32,
    decide_count: i32,
}

impl MenuController {
    pub fn update_decide(&mut self) -> MenuControllerUpdateDecideResult {
        if self.decide_count == 0 {
            return MenuControllerUpdateDecideResult::None;
        }
        if self.decide_count > 0 {
            self.decide_count += 1;
            if self.decide_count > 20 {
                self.decide_count = 0;
                return MenuControllerUpdateDecideResult::Decide;
            }
        } else {
            self.decide_count -= 1;
            if self.decide_count < -20 {
                self.decide_count = 0;
                return MenuControllerUpdateDecideResult::Cancel;
            }
        }
        MenuControllerUpdateDecideResult::Wait
    }

    fn cancel(
        &mut self,
        current_input: MenuButtons,
        prev_input: MenuButtons,
        instant: bool,
    ) -> bool {
        if !current_input.cancel_pulsed(prev_input) {
            return false;
        }
        if !instant {
            self.force_cancel();
        }
        true
    }

    pub fn force_cancel(&mut self) {
        self.decide_count = -1;
    }

    fn decide(
        &mut self,
        current_input: MenuButtons,
        prev_input: MenuButtons,
        instant: bool,
    ) -> bool {
        if !current_input.decide_pulsed(prev_input) {
            return false;
        }
        if !instant {
            self.decide_count = 1;
        }
        true
    }

    fn select(&mut self, current_input: MenuButtons, prev_input: MenuButtons) -> Option<bool> {
        let mut mv = None;
        if current_input.up && (!prev_input.up || self.repeat_up > 0) {
            if [0, 25].contains(&self.repeat_up) {
                mv = Some(true);
            }
            self.repeat_up += 1;
            if self.repeat_up > 25 {
                self.repeat_up = 17;
            }
        } else {
            self.repeat_up = 0;
        }
        if current_input.down && (!prev_input.down || self.repeat_down > 0) {
            if [0, 25].contains(&self.repeat_down) {
                mv = Some(false);
            }
            self.repeat_down += 1;
            if self.repeat_down > 25 {
                self.repeat_down = 17;
            }
        } else {
            self.repeat_down = 0;
        }
        mv
    }

    pub fn input(
        &mut self,
        current_input: MenuButtons,
        prev_input: MenuButtons,
        instant_decide: bool,
        instant_cancel: bool,
    ) -> MenuControllerInputResult {
        if self.cancel(current_input, prev_input, instant_cancel) {
            return MenuControllerInputResult::Cancel;
        }
        if self.decide(current_input, prev_input, instant_decide) {
            return MenuControllerInputResult::Decide;
        }
        if let Some(up) = self.select(current_input, prev_input) {
            if up {
                return MenuControllerInputResult::Up;
            } else {
                return MenuControllerInputResult::Down;
            }
        }
        MenuControllerInputResult::None
    }
}
