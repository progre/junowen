mod components;

use std::ffi::c_void;

use getset::{CopyGetters, Setters};
use junowen_lib::{InputFlags, InputValue, Th19};

use self::components::{Action, TextInput};

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

fn pulse(current: InputValue, prev: InputValue, flag: InputFlags) -> bool {
    current.0 & flag != None && prev.0 & flag == None
}

fn cancel(current_input: InputValue, prev_input: InputValue) -> bool {
    pulse(current_input, prev_input, InputFlags::CHARGE)
        || pulse(current_input, prev_input, InputFlags::BOMB)
        || pulse(current_input, prev_input, InputFlags::PAUSE)
}

enum CurrentMenuSceneResult<'a> {
    Menu(&'static str, &'a MenuDefine),
    SubScene(&'static str, LobbyScene),
    TextInput(&'static str, &'a TextInput),
}

enum CurrentMenuSceneMutResult<'a> {
    Menu(&'static str, &'a mut MenuDefine, &'a mut u32, &'a mut u32),
    SubScene(&'static str, LobbyScene),
    TextInput(&'static str, &'a mut TextInput),
}

#[derive(Setters, CopyGetters)]
pub struct CommonMenu {
    #[get_copy = "pub"]
    root_label: &'static str,
    menu_define: MenuDefine,
    instant_exit: bool,
    base_height: u32,
    depth: u32,
    repeat_up: u32,
    repeat_down: u32,
    decide_count: i32,
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
            depth: 0,
            repeat_up: 0,
            repeat_down: 0,
            decide_count: 0,
        }
    }

    fn apply_decide_count(&mut self) -> Option<OnMenuInputResult> {
        if self.decide_count == 0 {
            return None;
        }
        if self.decide_count > 0 {
            self.decide_count += 1;
            if self.decide_count <= 20 {
                return Some(OnMenuInputResult::None);
            }
            self.decide_count = 0;
            self.depth += 1;
            if let CurrentMenuSceneResult::SubScene(_, scene) = self.current_menu_scene() {
                self.depth -= 1;
                return Some(OnMenuInputResult::SubScene(scene));
            }
        } else {
            self.decide_count -= 1;
            if self.decide_count >= -20 {
                return Some(OnMenuInputResult::None);
            }
            self.decide_count = 0;
            if self.depth == 0 {
                return Some(OnMenuInputResult::Cancel);
            }
            self.depth -= 1;
        }
        None
    }

    fn cancel(&mut self, th19: &Th19) -> OnMenuInputResult {
        if self.depth == 0 && self.instant_exit {
            OnMenuInputResult::Cancel
        } else {
            th19.play_sound(th19.sound_manager(), 0x09, 0);
            self.decide_count -= 1;
            OnMenuInputResult::None
        }
    }

    fn decide(decide_count: &mut i32, th19: &Th19, menu_item: &MenuItem) -> OnMenuInputResult {
        if let Some(action) = menu_item.action() {
            if action.play_sound() {
                th19.play_sound(th19.sound_manager(), 0x07, 0);
            }
            return OnMenuInputResult::Action(action.clone());
        }
        match menu_item.child() {
            Some(MenuChild::SubMenu(_) | MenuChild::SubScene(_) | MenuChild::TextInput(_)) => {
                th19.play_sound(th19.sound_manager(), 0x07, 0);
                *decide_count += 1;
            }
            None => {}
        }
        OnMenuInputResult::None
    }

    fn select(
        menu: &mut MenuDefine,
        repeat_up: &mut u32,
        repeat_down: &mut u32,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
    ) {
        if current_input.0 & InputFlags::UP != None
            && (prev_input.0 & InputFlags::UP == None || *repeat_up > 0)
        {
            if [0, 25].contains(repeat_up) && menu.cursor() > 0 {
                menu.set_cursor(menu.cursor() - 1);
                th19.play_sound(th19.sound_manager(), 0x0a, 0);
            }
            *repeat_up += 1;
            if *repeat_up > 25 {
                *repeat_up = 17;
            }
        } else {
            *repeat_up = 0;
        }
        if current_input.0 & InputFlags::DOWN != None
            && (prev_input.0 & InputFlags::DOWN == None || *repeat_down > 0)
        {
            if [0, 25].contains(repeat_down) && menu.cursor() < menu.items().len() - 1 {
                menu.set_cursor(menu.cursor() + 1);
                th19.play_sound(th19.sound_manager(), 0x0a, 0);
            }
            *repeat_down += 1;
            if *repeat_down > 25 {
                *repeat_down = 17;
            }
        } else {
            *repeat_down = 0;
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
            CurrentMenuSceneResult::SubScene(_, scene) => {
                if cancel(current_input, prev_input) {
                    return self.cancel(th19);
                }
                OnMenuInputResult::SubScene(scene)
            }
            CurrentMenuSceneResult::Menu(_, menu) => {
                if cancel(current_input, prev_input) {
                    return self.cancel(th19);
                }
                if menu.items().is_empty() {
                    return OnMenuInputResult::None;
                }
                if pulse(current_input, prev_input, InputFlags::SHOT)
                    || pulse(current_input, prev_input, InputFlags::ENTER)
                {
                    let mut decide_count = self.decide_count;
                    let result = Self::decide(&mut decide_count, th19, menu.selected_item());
                    self.decide_count = decide_count;
                    return result;
                }
                let CurrentMenuSceneMutResult::Menu(_, menu, repeat_up, repeat_down) =
                    self.current_menu_scene_mut()
                else {
                    unreachable!()
                };
                Self::select(
                    menu,
                    repeat_up,
                    repeat_down,
                    current_input,
                    prev_input,
                    th19,
                );
                OnMenuInputResult::None
            }
            CurrentMenuSceneResult::TextInput(_, _) => {
                let CurrentMenuSceneMutResult::TextInput(_, text_input) =
                    self.current_menu_scene_mut()
                else {
                    unreachable!()
                };
                let action_id = text_input.id();
                match text_input.on_input_menu(th19) {
                    components::OnMenuInputResult::None => OnMenuInputResult::None,
                    components::OnMenuInputResult::Cancel => {
                        th19.play_sound(th19.sound_manager(), 0x09, 0);
                        self.decide_count -= 1;
                        OnMenuInputResult::None
                    }
                    components::OnMenuInputResult::Decide(new_room_name) => {
                        th19.play_sound(th19.sound_manager(), 0x07, 0);
                        self.decide_count -= 1;
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
        if self.depth == 0 {
            return CurrentMenuSceneResult::Menu(self.root_label, &self.menu_define);
        }
        let item = self.menu_define.selected_item();
        let mut label = item.label();
        let mut child = item.child().as_ref().unwrap();
        for _ in 1..self.depth {
            let MenuChild::SubMenu(sub_menu) = child else {
                unreachable!()
            };
            let item = sub_menu.selected_item();
            label = item.label();
            child = item.child().as_ref().unwrap();
        }
        match child {
            MenuChild::SubMenu(sub_menu) => CurrentMenuSceneResult::Menu(label, sub_menu),
            MenuChild::SubScene(scene) => CurrentMenuSceneResult::SubScene(label, *scene),
            MenuChild::TextInput(text_input) => {
                CurrentMenuSceneResult::TextInput(label, text_input)
            }
        }
    }

    fn current_menu_scene_mut(&mut self) -> CurrentMenuSceneMutResult {
        if self.depth == 0 {
            return CurrentMenuSceneMutResult::Menu(
                self.root_label,
                &mut self.menu_define,
                &mut self.repeat_up,
                &mut self.repeat_down,
            );
        }
        let item = self.menu_define.selected_item_mut();
        let mut label = item.label();
        let mut child = item.child_mut().as_mut().unwrap();
        for _ in 1..self.depth {
            let MenuChild::SubMenu(sub_menu) = child else {
                unreachable!()
            };
            let item = sub_menu.selected_item_mut();
            label = item.label();
            child = item.child_mut().as_mut().unwrap();
        }
        match child {
            MenuChild::SubMenu(sub_menu) => CurrentMenuSceneMutResult::Menu(
                label,
                sub_menu,
                &mut self.repeat_up,
                &mut self.repeat_down,
            ),
            MenuChild::SubScene(scene) => CurrentMenuSceneMutResult::SubScene(label, *scene),
            MenuChild::TextInput(text_input) => {
                CurrentMenuSceneMutResult::TextInput(label, text_input)
            }
        }
    }
}
