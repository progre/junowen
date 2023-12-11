mod components;
mod menu_controller;

use std::ffi::c_void;

use getset::{CopyGetters, Setters};
use junowen_lib::{InputValue, Th19};

use self::{
    components::{Action, TextInput},
    menu_controller::{
        MenuController, MenuControllerInputResult, MenuControllerUpdateDecideResult,
    },
};

use super::helper::render_title;

pub use components::{MenuChild, MenuDefine, MenuItem};

#[derive(Clone, Copy, Debug)]
pub enum LobbyScene {
    Root,
    SharedRoom,
    ReservedRoom,
    PureP2pHost,
    PureP2pGuest,
    PureP2pSpectator,
}

pub enum OnMenuInputResult {
    None,
    Cancel,
    Action(Action),
    SubScene(LobbyScene),
}

enum CurrentMenuSceneResult<'a> {
    Menu(&'static str, &'a MenuDefine),
    SubScene(&'static str, LobbyScene),
    TextInput(&'static str, &'a TextInput),
}

enum CurrentMenuSceneMutResult<'a> {
    Menu(&'a mut MenuDefine),
    SubScene(LobbyScene),
    TextInput(&'a mut TextInput),
}

#[derive(Setters, CopyGetters)]
pub struct CommonMenu {
    #[get_copy = "pub"]
    root_label: &'static str,
    menu_define: MenuDefine,
    instant_exit: bool,
    base_height: u32,
    controller: MenuController,
}

impl CommonMenu {
    pub fn new(
        root_label: &'static str,
        instant_exit: bool,
        base_height: u32,
        menu_define: MenuDefine,
    ) -> Self {
        Self {
            root_label,
            menu_define,
            instant_exit,
            base_height,
            controller: MenuController::default(),
        }
    }

    fn apply_decide_count(&mut self) -> Option<OnMenuInputResult> {
        match self.controller.update_decide() {
            MenuControllerUpdateDecideResult::None => None,
            MenuControllerUpdateDecideResult::Wait => Some(OnMenuInputResult::None),
            MenuControllerUpdateDecideResult::Decide => {
                if let Some(scene) = self.menu_define.dig() {
                    return Some(OnMenuInputResult::SubScene(scene));
                }
                None
            }
            MenuControllerUpdateDecideResult::Cancel => {
                if !self.menu_define.bury() {
                    return Some(OnMenuInputResult::Cancel);
                }
                None
            }
        }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
    ) -> OnMenuInputResult {
        if let Some(result) = self.apply_decide_count() {
            return result;
        }

        match self.current_menu_scene() {
            CurrentMenuSceneResult::SubScene(_, scene) => OnMenuInputResult::SubScene(scene),
            CurrentMenuSceneResult::Menu(_, menu) => {
                let ignore_decide = menu.items().is_empty();
                let instant_decide = ignore_decide || menu.selected_item().child().is_none();
                let play_decide_sound = !ignore_decide
                    && menu
                        .selected_item()
                        .action()
                        .map(|x| x.play_sound())
                        .unwrap_or(true);
                let root_cancel = !self.menu_define.decided() && self.instant_exit;

                let input_result =
                    self.controller
                        .input(current_input, prev_input, instant_decide, root_cancel);

                if root_cancel {
                    if let MenuControllerInputResult::Cancel = input_result {
                        return OnMenuInputResult::Cancel;
                    }
                }
                let CurrentMenuSceneMutResult::Menu(menu) = self.current_menu_scene_mut() else {
                    unreachable!()
                };
                if let Some(action) =
                    menu.input(input_result, ignore_decide, play_decide_sound, th19)
                {
                    OnMenuInputResult::Action(action)
                } else {
                    OnMenuInputResult::None
                }
            }
            CurrentMenuSceneResult::TextInput(_, _) => {
                let CurrentMenuSceneMutResult::TextInput(text_input) =
                    self.current_menu_scene_mut()
                else {
                    unreachable!()
                };
                let action_id = text_input.id();
                match text_input.on_input_menu(th19) {
                    components::OnMenuInputResult::None => OnMenuInputResult::None,
                    components::OnMenuInputResult::Cancel => {
                        th19.play_sound(th19.sound_manager(), 0x09, 0);
                        self.controller.force_cancel();
                        OnMenuInputResult::None
                    }
                    components::OnMenuInputResult::Decide(new_room_name) => {
                        th19.play_sound(th19.sound_manager(), 0x07, 0);
                        self.controller.force_cancel();
                        let action = Action::new(action_id, false, Some(new_room_name));
                        OnMenuInputResult::Action(action)
                    }
                }
            }
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        match self.current_menu_scene() {
            CurrentMenuSceneResult::SubScene { .. } => unreachable!(),
            CurrentMenuSceneResult::Menu(label, menu) => {
                render_title(th19, text_renderer, label.as_bytes());
                menu.on_render_texts(self.base_height, th19, text_renderer);
            }
            CurrentMenuSceneResult::TextInput(label, text_input) => {
                render_title(th19, text_renderer, label.as_bytes());
                text_input.on_render_texts(th19, text_renderer);
            }
        }
    }

    fn current_menu_scene(&self) -> CurrentMenuSceneResult {
        if !self.menu_define.decided() {
            return CurrentMenuSceneResult::Menu(self.root_label, &self.menu_define);
        }
        let mut menu = &self.menu_define;
        loop {
            let decided_item = menu.selected_item();
            let label = decided_item.label();
            let child = decided_item.child().unwrap();
            match child {
                MenuChild::SubMenu(sub_menu) => {
                    if sub_menu.decided() {
                        menu = sub_menu;
                        continue;
                    }
                    return CurrentMenuSceneResult::Menu(label, sub_menu);
                }
                MenuChild::SubScene(scene) => {
                    return CurrentMenuSceneResult::SubScene(label, *scene);
                }
                MenuChild::TextInput(text_input) => {
                    return CurrentMenuSceneResult::TextInput(label, text_input);
                }
            }
        }
    }

    fn current_menu_scene_mut(&mut self) -> CurrentMenuSceneMutResult {
        if !self.menu_define.decided() {
            return CurrentMenuSceneMutResult::Menu(&mut self.menu_define);
        }
        let mut menu = &mut self.menu_define;
        loop {
            let decided_item = menu.selected_item_mut();
            let child = decided_item.child_mut().unwrap();
            match child {
                MenuChild::SubMenu(sub_menu) => {
                    if sub_menu.decided() {
                        menu = sub_menu;
                        continue;
                    }
                    return CurrentMenuSceneMutResult::Menu(sub_menu);
                }
                MenuChild::SubScene(scene) => {
                    return CurrentMenuSceneMutResult::SubScene(*scene);
                }
                MenuChild::TextInput(text_input) => {
                    return CurrentMenuSceneMutResult::TextInput(text_input);
                }
            }
        }
    }
}
