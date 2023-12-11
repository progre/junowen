use std::{ffi::c_void, num::NonZeroU8};

use getset::Setters;
use junowen_lib::Th19;
use windows::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyW, ToUnicode, MAPVK_VK_TO_VSC};

use crate::in_game_lobby::helper::render_label_value;

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
    Decide(u8, String),
}

#[derive(Debug, Setters)]
pub struct TextInput {
    changed_action: u8,
    name: &'static str,
    #[getset(set = "pub")]
    value: String,
    state: TextInputState,
}

impl TextInput {
    pub fn new(changed_action: u8, name: &'static str) -> Self {
        Self {
            changed_action,
            name,
            value: String::new(),
            state: TextInputState::new(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
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
                return OnMenuInputResult::Decide(self.changed_action, self.value.clone());
            } else if ascii == 0x1b {
                // ESC
                return OnMenuInputResult::Cancel;
            }
        }
        OnMenuInputResult::None
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        render_label_value(th19, text_renderer, 480, 0, self.name, &self.value);
    }
}
