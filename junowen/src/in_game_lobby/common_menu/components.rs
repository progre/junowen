use std::{ffi::c_void, num::NonZeroU8};

use derive_new::new;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use junowen_lib::Th19;
use windows::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyW, ToUnicode, MAPVK_VK_TO_VSC};

use crate::in_game_lobby::helper::{render_label_value, render_menu_item};

use super::{menu_controller::MenuControllerInputResult, LobbyScene};

#[derive(Clone, Debug, CopyGetters, Getters, new)]
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

fn to_ascii(vk: u32, current: &[u8; 256]) -> Option<NonZeroU8> {
    let mut buf = [0u16; 2];
    unsafe {
        let scan_code = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
        ToUnicode(vk, scan_code, Some(current), &mut buf, 0);
    }
    NonZeroU8::new(String::from_utf16_lossy(&buf).as_bytes()[0])
}

#[derive(Debug)]
struct TextInputState {
    prev: [u8; 256],
    current_vk: u8,
    current_vk_count: u32,
}

impl TextInputState {
    pub fn new() -> Self {
        Self {
            prev: [0; 256],
            current_vk: 0,
            current_vk_count: 0,
        }
    }

    pub fn tick(&mut self, current: &[u8; 256]) -> Vec<u8> {
        let mut result = vec![];
        for (vk, _) in current
            .iter()
            .enumerate()
            .filter(|&(vk, value)| value & 0x80 != 0 && self.prev[vk] & 0x80 == 0)
        {
            if let Some(ascii) = to_ascii(vk as u32, current) {
                result.push(ascii.get());
            }
            let vk = vk as u8;
            if vk != self.current_vk {
                self.current_vk = vk;
                self.current_vk_count = 0;
            }
        }
        if current[self.current_vk as usize] & 0x80 != 0 {
            if self.current_vk_count > 30 {
                if let Some(ascii) = to_ascii(self.current_vk as u32, current) {
                    result.push(ascii.get());
                }
            }
            self.current_vk_count += 1;
        } else {
            self.current_vk_count = 0;
        }
        self.prev.copy_from_slice(current);
        result
    }
}

pub enum OnMenuInputResult {
    None,
    Cancel,
    Decide(String),
}

#[derive(Debug, CopyGetters)]
pub struct TextInput {
    #[get_copy = "pub"]
    id: u8,
    name: &'static str,
    default_value: String,
    value: String,
    state: TextInputState,
}

impl TextInput {
    pub fn new(id: u8, name: &'static str, default_value: String) -> Self {
        Self {
            id,
            name,
            default_value: default_value.clone(),
            value: default_value,
            state: TextInputState::new(),
        }
    }

    pub fn on_input_menu(&mut self, th19: &Th19) -> OnMenuInputResult {
        for ascii in self
            .state
            .tick(th19.input_devices().keyboard_input().raw_keys())
        {
            if (0x20..0x7f).contains(&ascii) {
                self.value.push(ascii as char);
            } else if ascii == 0x08 {
                self.value.pop();
            } else if ascii == 0x0d {
                // CR
                return OnMenuInputResult::Decide(self.value.clone());
            } else if ascii == 0x1b {
                // ESC
                self.value = self.default_value.clone();
                return OnMenuInputResult::Cancel;
            }
        }
        OnMenuInputResult::None
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        render_label_value(th19, text_renderer, 480, 0, self.name, &self.value);
    }
}

#[derive(Debug)]
pub enum MenuChild {
    SubMenu(MenuDefine),
    SubScene(LobbyScene),
    TextInput(Box<TextInput>),
}

#[derive(Debug, CopyGetters, Getters, MutGetters)]
pub struct MenuItem {
    #[get_copy = "pub"]
    label: &'static str,
    action: Option<Action>,
    child: Option<MenuChild>,
}

impl MenuItem {
    pub fn new(label: &'static str, action: Option<Action>, child: Option<MenuChild>) -> Self {
        Self {
            label,
            action,
            child,
        }
    }

    pub fn simple_action(label: &'static str, id: u8, play_sound: bool) -> Self {
        Self::new(label, Some(Action::new(id, play_sound, None)), None)
    }

    pub fn simple_sub_scene(label: &'static str, scene: LobbyScene) -> Self {
        Self::new(label, None, Some(MenuChild::SubScene(scene)))
    }

    pub fn text_input(label: &'static str, id: u8, name: &'static str, value: String) -> Self {
        let menu_child = MenuChild::TextInput(Box::new(TextInput::new(id, name, value)));
        Self::new(label, None, Some(menu_child))
    }

    pub fn action(&self) -> Option<&Action> {
        self.action.as_ref()
    }

    pub fn child(&self) -> Option<&MenuChild> {
        self.child.as_ref()
    }

    pub fn child_mut(&mut self) -> Option<&mut MenuChild> {
        self.child.as_mut()
    }
}

#[derive(Debug, CopyGetters, Getters, Setters)]
pub struct MenuDefine {
    #[get_copy = "pub"]
    cursor: usize,
    #[get_copy = "pub"]
    decided: bool,
    #[get = "pub"]
    items: Vec<MenuItem>,
}

impl MenuDefine {
    pub fn new(cursor: usize, items: Vec<MenuItem>) -> Self {
        Self {
            cursor,
            decided: false,
            items,
        }
    }

    pub fn selected_item(&self) -> &MenuItem {
        &self.items[self.cursor]
    }

    pub fn selected_item_mut(&mut self) -> &mut MenuItem {
        &mut self.items[self.cursor]
    }

    pub fn dig(&mut self) -> Option<LobbyScene> {
        if !self.decided {
            if let Some(MenuChild::SubScene(scene)) = self.selected_item().child() {
                return Some(*scene);
            }
            self.decided = true;
            return None;
        }
        let MenuChild::SubMenu(sub_menu) = self.selected_item_mut().child_mut().unwrap() else {
            unreachable!()
        };
        sub_menu.dig()
    }

    pub fn bury(&mut self) -> bool {
        if !self.decided {
            return false;
        }
        let MenuChild::SubMenu(sub_menu) = self.selected_item_mut().child_mut().unwrap() else {
            self.decided = false;
            return true;
        };
        if !sub_menu.decided {
            self.decided = false;
            return true;
        }
        sub_menu.bury()
    }

    pub fn on_render_texts(&self, base_height: u32, th19: &Th19, text_renderer: *const c_void) {
        for (i, item) in self.items().iter().enumerate() {
            let label = item.label().as_bytes();
            let height = base_height + 56 * i as u32;
            render_menu_item(th19, text_renderer, label, height, i == self.cursor());
        }
    }

    fn increment_cursor(&mut self) -> bool {
        if self.cursor() >= self.items().len() - 1 {
            return false;
        }
        self.cursor += 1;
        true
    }

    fn decrement_cursor(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    pub fn input(
        &mut self,
        input_result: MenuControllerInputResult,
        ignore_decide: bool,
        play_decide_sound: bool,
        th19: &Th19,
    ) -> Option<Action> {
        match input_result {
            MenuControllerInputResult::None => None,
            MenuControllerInputResult::Cancel => {
                th19.play_sound(th19.sound_manager(), 0x09, 0);
                None
            }
            MenuControllerInputResult::Decide => {
                if ignore_decide {
                    return None;
                }
                if play_decide_sound {
                    th19.play_sound(th19.sound_manager(), 0x07, 0);
                }
                self.selected_item().action().cloned()
            }
            MenuControllerInputResult::Up => {
                if !self.decrement_cursor() {
                    return None;
                }
                th19.play_sound(th19.sound_manager(), 0x0a, 0);
                None
            }
            MenuControllerInputResult::Down => {
                if !self.increment_cursor() {
                    return None;
                }
                th19.play_sound(th19.sound_manager(), 0x0a, 0);
                None
            }
        }
    }
}
