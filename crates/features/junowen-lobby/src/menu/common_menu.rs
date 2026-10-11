use getset::{Getters, MutGetters};

use crate::menu::{
    action::{Action, OnMenuInputResult},
    device::{MenuDevice, MenuSound},
    input::{
        MenuButtons, MenuController, MenuControllerInputResult, MenuControllerUpdateDecideResult,
        OnTextInputResult,
    },
    render::{MenuRenderer, render_title},
    tree::{
        CurrentMenuSceneMutResult, CurrentMenuSceneResult, Menu, MenuItem, current_menu_scene,
        current_menu_scene_mut,
    },
};

#[derive(Getters, MutGetters)]
pub struct CommonMenu<S> {
    #[getset(get = "pub", get_mut = "pub")]
    menu: Menu<S>,
    instant_exit: bool,
    base_height: u32,
    #[getset(get_mut = "pub")]
    controller: MenuController,
}

impl<S: Copy> CommonMenu<S> {
    pub fn new(instant_exit: bool, base_height: u32, menu: Menu<S>) -> Self {
        Self {
            menu,
            instant_exit,
            base_height,
            controller: MenuController::default(),
        }
    }

    pub fn root_title(&self) -> &'static str {
        self.menu.title()
    }

    fn apply_decide_count(&mut self) -> Option<OnMenuInputResult<S>> {
        match self.controller.update_decide() {
            MenuControllerUpdateDecideResult::None => None,
            MenuControllerUpdateDecideResult::Wait => Some(OnMenuInputResult::None),
            MenuControllerUpdateDecideResult::Decide => {
                if let Some(scene) = self.menu.dig() {
                    return Some(OnMenuInputResult::SubScene(scene));
                }
                None
            }
            MenuControllerUpdateDecideResult::Cancel => {
                if !self.menu.bury() {
                    return Some(OnMenuInputResult::Cancel);
                }
                None
            }
        }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: MenuButtons,
        prev_input: MenuButtons,
        device: &impl MenuDevice,
    ) -> OnMenuInputResult<S> {
        if let Some(result) = self.apply_decide_count() {
            return result;
        }

        match current_menu_scene(&self.menu) {
            CurrentMenuSceneResult::SubScene(scene) => OnMenuInputResult::SubScene(scene),
            CurrentMenuSceneResult::Menu(menu) => {
                let ignore_decide = menu.items().is_empty();
                let instant_decide =
                    ignore_decide || matches!(menu.selected_item(), MenuItem::Plain(_));
                let play_decide_sound = !ignore_decide
                    && menu
                        .selected_item()
                        .decided_action()
                        .map(|x| x.play_sound())
                        .unwrap_or(true);
                let root_cancel = !self.menu.decided() && self.instant_exit;

                let input_result =
                    self.controller
                        .input(current_input, prev_input, instant_decide, root_cancel);

                if root_cancel && let MenuControllerInputResult::Cancel = input_result {
                    return OnMenuInputResult::Cancel;
                }
                let CurrentMenuSceneMutResult::Menu(menu) = current_menu_scene_mut(&mut self.menu)
                else {
                    unreachable!()
                };
                if let Some(action) =
                    menu.input(input_result, ignore_decide, play_decide_sound, device)
                {
                    OnMenuInputResult::Action(action)
                } else {
                    OnMenuInputResult::None
                }
            }
            CurrentMenuSceneResult::TextInput(_, _) => {
                let CurrentMenuSceneMutResult::TextInput(text_input) =
                    current_menu_scene_mut(&mut self.menu)
                else {
                    unreachable!()
                };
                match text_input.on_input_menu(device) {
                    OnTextInputResult::None => OnMenuInputResult::None,
                    OnTextInputResult::Cancel => {
                        device.play_sound(MenuSound::Cancel);
                        self.controller.force_cancel();
                        OnMenuInputResult::None
                    }
                    OnTextInputResult::Decide(changed_action, new_room_name) => {
                        device.play_sound(MenuSound::Decide);
                        self.controller.force_cancel();
                        let action = Action::new(changed_action, false, Some(new_room_name));
                        OnMenuInputResult::Action(action)
                    }
                }
            }
        }
    }

    pub fn on_render_texts(&self, renderer: &impl MenuRenderer) {
        match current_menu_scene(&self.menu) {
            CurrentMenuSceneResult::SubScene { .. } => unreachable!(),
            CurrentMenuSceneResult::Menu(menu) => {
                render_title(renderer, menu.title().as_bytes());
                menu.on_render_texts(self.base_height, renderer);
            }
            CurrentMenuSceneResult::TextInput(label, text_input) => {
                render_title(renderer, label.as_bytes());
                text_input.on_render_texts(renderer);
            }
        }
    }
}
