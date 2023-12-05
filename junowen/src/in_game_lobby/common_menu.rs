mod components;

use std::ffi::c_void;

use getset::{CopyGetters, Setters};
use junowen_lib::{InputFlags, InputValue, Th19};

use self::components::Action;

use super::helper::{render_menu_item, render_title};

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

enum CurrentMenuResult<'a> {
    MenuDefine(&'a MenuDefine),
    SubScene(LobbyScene),
}
enum CurrentMenuMutResult<'a> {
    MenuDefine(&'a mut MenuDefine, &'a mut u32, &'a mut u32),
    SubScene(LobbyScene),
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
            if let (_, CurrentMenuResult::SubScene(scene)) = self.current_menu() {
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
            Some(MenuChild::SubMenu(_)) => {
                th19.play_sound(th19.sound_manager(), 0x07, 0);
                *decide_count += 1;
            }
            Some(MenuChild::SubScene(_)) => {
                th19.play_sound(th19.sound_manager(), 0x07, 0);
                *decide_count += 1;
            }
            None => {}
        }
        OnMenuInputResult::None
    }

    fn select(&mut self, current_input: InputValue, prev_input: InputValue, th19: &Th19) {
        let (_label, result) = self.current_menu_mut();
        let (menu, repeat_up, repeat_down) = match result {
            CurrentMenuMutResult::SubScene(_) => unreachable!(),
            CurrentMenuMutResult::MenuDefine(menu, repeat_up, repeat_down) => {
                (menu, repeat_up, repeat_down)
            }
        };

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

        if pulse(current_input, prev_input, InputFlags::CHARGE)
            || pulse(current_input, prev_input, InputFlags::BOMB)
            || pulse(current_input, prev_input, InputFlags::PAUSE)
        {
            return self.cancel(th19);
        }
        let (_label, result) = self.current_menu();
        let menu = match result {
            CurrentMenuResult::SubScene(scene) => return OnMenuInputResult::SubScene(scene),
            CurrentMenuResult::MenuDefine(menu) => menu,
        };
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
        self.select(current_input, prev_input, th19);
        OnMenuInputResult::None
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        let (label, menu) = self.current_menu();
        let menu = match menu {
            CurrentMenuResult::SubScene(_) => unreachable!(),
            CurrentMenuResult::MenuDefine(menu) => menu,
        };

        render_title(th19, text_renderer, label.as_bytes());
        for (i, item) in menu.items().iter().enumerate() {
            render_menu_item(
                th19,
                text_renderer,
                item.label().as_bytes(),
                self.base_height + 56 * i as u32,
                i == menu.cursor(),
            );
        }
    }

    fn current_menu(&self) -> (&'static str, CurrentMenuResult) {
        if self.depth == 0 {
            return (
                self.root_label,
                CurrentMenuResult::MenuDefine(&self.menu_define),
            );
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
        (
            label,
            match child {
                MenuChild::SubMenu(sub_menu) => CurrentMenuResult::MenuDefine(sub_menu),
                MenuChild::SubScene(scene) => CurrentMenuResult::SubScene(*scene),
            },
        )
    }

    fn current_menu_mut(&mut self) -> (&'static str, CurrentMenuMutResult) {
        if self.depth == 0 {
            return (
                self.root_label,
                CurrentMenuMutResult::MenuDefine(
                    &mut self.menu_define,
                    &mut self.repeat_up,
                    &mut self.repeat_down,
                ),
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
        (
            label,
            match child {
                MenuChild::SubMenu(sub_menu) => CurrentMenuMutResult::MenuDefine(
                    sub_menu,
                    &mut self.repeat_up,
                    &mut self.repeat_down,
                ),
                MenuChild::SubScene(scene) => CurrentMenuMutResult::SubScene(*scene),
            },
        )
    }
}
