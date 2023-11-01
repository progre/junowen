use std::ffi::c_void;

use getset::Setters;
use junowen_lib::{InputFlags, InputValue, Th19};

use super::helper::{render_menu_item, render_title};

#[derive(Clone, Copy, Debug)]
pub enum LobbyScene {
    Root,
    PureP2pHost,
    PureP2pGuest,
    PureP2pSpectator,
}

impl From<LobbyScene> for MenuContent {
    fn from(value: LobbyScene) -> Self {
        MenuContent::Action(MenuAction::SubScene(value))
    }
}

#[derive(Debug)]
pub enum MenuAction {
    Action(u8, bool),
    SubScene(LobbyScene),
}

#[derive(Debug)]
pub enum MenuContent {
    Action(MenuAction),
    _SubMenu(MenuDefine),
}

impl From<MenuAction> for MenuContent {
    fn from(value: MenuAction) -> Self {
        MenuContent::Action(value)
    }
}

pub enum OnMenuInputResult {
    None,
    Cancel,
    Action(MenuAction),
}

#[derive(Debug)]
pub struct MenuItem {
    label: &'static str,
    content: MenuContent,
}

impl MenuItem {
    pub fn new(label: &'static str, content: MenuContent) -> Self {
        Self { label, content }
    }
}

#[derive(Debug)]
pub struct MenuDefine {
    items: Vec<MenuItem>,
    cursor: usize,
}

impl MenuDefine {
    pub fn new(cursor_default: usize, items: Vec<MenuItem>) -> Self {
        Self {
            items,
            cursor: cursor_default,
        }
    }
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

#[derive(Setters)]
pub struct CommonMenu {
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

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
    ) -> OnMenuInputResult {
        if self.decide_count != 0 {
            if self.decide_count > 0 {
                self.decide_count += 1;
                if self.decide_count <= 25 {
                    return OnMenuInputResult::None;
                }
                self.decide_count = 0;
                self.depth += 1;
                if let (_, CurrentMenuResult::SubScene(scene)) = self.current_menu() {
                    self.depth -= 1;
                    return OnMenuInputResult::Action(MenuAction::SubScene(scene));
                }
            } else {
                self.decide_count -= 1;
                if self.decide_count >= -25 {
                    return OnMenuInputResult::None;
                }
                self.decide_count = 0;
                if self.depth == 0 {
                    return OnMenuInputResult::Cancel;
                }
                self.depth -= 1;
            }
        }

        if pulse(current_input, prev_input, InputFlags::CHARGE)
            || pulse(current_input, prev_input, InputFlags::BOMB)
            || pulse(current_input, prev_input, InputFlags::PAUSE)
        {
            if self.depth == 0 && self.instant_exit {
                return OnMenuInputResult::Cancel;
            } else {
                th19.play_sound(th19.sound_manager(), 0x09, 0);
                self.decide_count -= 1;
                return OnMenuInputResult::None;
            }
        }
        let (_label, result) = self.current_menu_mut();
        let (menu, repeat_up, repeat_down) = match result {
            CurrentMenuMutResult::SubScene(scene) => {
                return OnMenuInputResult::Action(MenuAction::SubScene(scene))
            }
            CurrentMenuMutResult::MenuDefine(menu, repeat_up, repeat_down) => {
                (menu, repeat_up, repeat_down)
            }
        };
        if menu.items.is_empty() {
            return OnMenuInputResult::None;
        }
        if pulse(current_input, prev_input, InputFlags::SHOT) {
            match menu.items[menu.cursor].content {
                MenuContent::_SubMenu(_) => {
                    th19.play_sound(th19.sound_manager(), 0x07, 0);
                    self.decide_count += 1;
                    return OnMenuInputResult::None;
                }
                MenuContent::Action(MenuAction::SubScene(_)) => {
                    th19.play_sound(th19.sound_manager(), 0x07, 0);
                    self.decide_count += 1;
                    return OnMenuInputResult::None;
                }
                MenuContent::Action(MenuAction::Action(action, sound)) => {
                    if sound {
                        th19.play_sound(th19.sound_manager(), 0x07, 0);
                    }
                    return OnMenuInputResult::Action(MenuAction::Action(action, sound));
                }
            }
        }
        if current_input.0 & InputFlags::UP != None
            && (prev_input.0 & InputFlags::UP == None || *repeat_up > 0)
        {
            if [0, 25].contains(repeat_up) && menu.cursor > 0 {
                menu.cursor -= 1;
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
            if [0, 25].contains(repeat_down) && menu.cursor < menu.items.len() - 1 {
                menu.cursor += 1;
                th19.play_sound(th19.sound_manager(), 0x0a, 0);
            }
            *repeat_down += 1;
            if *repeat_down > 25 {
                *repeat_down = 17;
            }
        } else {
            *repeat_down = 0;
        }
        OnMenuInputResult::None
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        let (label, menu) = self.current_menu();
        let menu = match menu {
            CurrentMenuResult::SubScene(_) => unreachable!(),
            CurrentMenuResult::MenuDefine(menu) => menu,
        };

        render_title(th19, text_renderer, label.as_bytes());
        for (i, item) in menu.items.iter().enumerate() {
            render_menu_item(
                th19,
                text_renderer,
                item.label.as_bytes(),
                self.base_height + 56 * i as u32,
                i == menu.cursor,
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
        let item = &self.menu_define.items[self.menu_define.cursor];
        let mut label = item.label;
        let mut content = &item.content;
        for _ in 1..self.depth {
            let MenuContent::_SubMenu(sub_menu) = content else {
                unreachable!()
            };
            let item = &sub_menu.items[sub_menu.cursor];
            label = item.label;
            content = &item.content;
        }
        (
            label,
            match content {
                MenuContent::_SubMenu(sub_menu) => CurrentMenuResult::MenuDefine(sub_menu),
                MenuContent::Action(MenuAction::SubScene(scene)) => {
                    CurrentMenuResult::SubScene(*scene)
                }
                MenuContent::Action(MenuAction::Action(..)) => unreachable!(),
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
        let item = &mut self.menu_define.items[self.menu_define.cursor];
        let mut label = item.label;
        let mut content = &mut item.content;
        for _ in 1..self.depth {
            let MenuContent::_SubMenu(sub_menu) = content else {
                unreachable!()
            };
            let item = &mut sub_menu.items[sub_menu.cursor];
            label = item.label;
            content = &mut item.content;
        }
        (
            label,
            match content {
                MenuContent::_SubMenu(sub_menu) => CurrentMenuMutResult::MenuDefine(
                    sub_menu,
                    &mut self.repeat_up,
                    &mut self.repeat_down,
                ),
                MenuContent::Action(MenuAction::SubScene(scene)) => {
                    CurrentMenuMutResult::SubScene(*scene)
                }
                MenuContent::Action(MenuAction::Action(..)) => unreachable!(),
            },
        )
    }
}
