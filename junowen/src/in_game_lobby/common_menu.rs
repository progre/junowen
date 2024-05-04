mod menu;
mod menu_controller;
mod menu_item;
mod text_input;

use std::ffi::c_void;

use derive_new::new;
use getset::{CopyGetters, Getters, MutGetters};
use junowen_lib::{structs::input_devices::InputValue, Th19};

use self::{
    menu_controller::{
        MenuController, MenuControllerInputResult, MenuControllerUpdateDecideResult,
    },
    text_input::TextInput,
};

use super::helper::render_title;

pub use {menu::Menu, menu_item::MenuItem};

#[derive(Debug, CopyGetters, new)]
pub struct Action {
    #[get_copy = "pub"]
    id: u8,
    #[get_copy = "pub"]
    play_sound: bool,
    value: Option<String>,
}

impl Action {
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

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

#[derive(Debug)]
enum CurrentMenuSceneResult<'a> {
    Menu(&'a Menu),
    SubScene(LobbyScene),
    TextInput(&'static str, &'a TextInput),
}

enum CurrentMenuSceneMutResult<'a> {
    Menu(&'a mut Menu),
    SubScene,
    TextInput(&'a mut TextInput),
}

#[derive(Getters, MutGetters)]
pub struct CommonMenu {
    #[getset(get = "pub", get_mut = "pub")]
    menu: Menu,
    instant_exit: bool,
    base_height: u32,
    #[getset(get_mut = "pub")]
    controller: MenuController,
}

impl CommonMenu {
    pub fn new(instant_exit: bool, base_height: u32, menu: Menu) -> Self {
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

    fn apply_decide_count(&mut self) -> Option<OnMenuInputResult> {
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
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
    ) -> OnMenuInputResult {
        if let Some(result) = self.apply_decide_count() {
            return result;
        }

        match self.current_menu_scene() {
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
                match text_input.on_input_menu(th19) {
                    text_input::OnMenuInputResult::None => OnMenuInputResult::None,
                    text_input::OnMenuInputResult::Cancel => {
                        th19.play_sound(th19.sound_manager(), 0x09, 0);
                        self.controller.force_cancel();
                        OnMenuInputResult::None
                    }
                    text_input::OnMenuInputResult::Decide(changed_action, new_room_name) => {
                        th19.play_sound(th19.sound_manager(), 0x07, 0);
                        self.controller.force_cancel();
                        let action = Action::new(changed_action, false, Some(new_room_name));
                        OnMenuInputResult::Action(action)
                    }
                }
            }
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        match self.current_menu_scene() {
            CurrentMenuSceneResult::SubScene { .. } => unreachable!(),
            CurrentMenuSceneResult::Menu(menu) => {
                render_title(th19, text_renderer, menu.title().as_bytes());
                menu.on_render_texts(self.base_height, th19, text_renderer);
            }
            CurrentMenuSceneResult::TextInput(label, text_input) => {
                render_title(th19, text_renderer, label.as_bytes());
                text_input.on_render_texts(th19, text_renderer);
            }
        }
    }

    fn current_menu_scene(&self) -> CurrentMenuSceneResult {
        if !self.menu.decided() {
            return CurrentMenuSceneResult::Menu(&self.menu);
        }
        let mut menu = &self.menu;
        loop {
            match menu.selected_item() {
                MenuItem::Plain(_) => unreachable!(),
                MenuItem::SubMenu(item) => {
                    if item.sub_menu().decided() {
                        menu = item.sub_menu();
                        continue;
                    }
                    return CurrentMenuSceneResult::Menu(item.sub_menu());
                }
                MenuItem::SubScene(item) => {
                    return CurrentMenuSceneResult::SubScene(item.sub_scene());
                }
                MenuItem::TextInput(item) => {
                    return CurrentMenuSceneResult::TextInput(item.label(), item.text_input());
                }
            }
        }
    }

    fn current_menu_scene_mut(&mut self) -> CurrentMenuSceneMutResult {
        if !self.menu.decided() {
            return CurrentMenuSceneMutResult::Menu(&mut self.menu);
        }
        let mut menu = &mut self.menu;
        loop {
            match menu.selected_item_mut() {
                MenuItem::Plain(_) => unreachable!(),
                MenuItem::SubMenu(item) => {
                    if item.sub_menu().decided() {
                        menu = item.sub_menu_mut();
                        continue;
                    }
                    return CurrentMenuSceneMutResult::Menu(item.sub_menu_mut());
                }
                MenuItem::SubScene(_item) => {
                    return CurrentMenuSceneMutResult::SubScene;
                }
                MenuItem::TextInput(item) => {
                    return CurrentMenuSceneMutResult::TextInput(item.text_input_mut());
                }
            }
        }
    }
}
