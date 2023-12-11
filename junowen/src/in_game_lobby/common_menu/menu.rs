use std::ffi::c_void;

use getset::{CopyGetters, Getters, Setters};
use junowen_lib::Th19;

use crate::in_game_lobby::helper::render_menu_item;

use super::{menu_controller::MenuControllerInputResult, menu_item::MenuItem, Action, LobbyScene};

#[derive(CopyGetters, Debug, Getters, Setters)]
pub struct Menu {
    #[get_copy = "pub"]
    title: &'static str,
    canceled_action: Option<u8>,
    #[get = "pub"]
    items: Vec<MenuItem>,
    #[get_copy = "pub"]
    cursor: usize,
    #[get_copy = "pub"]
    decided: bool,
}

impl Menu {
    pub fn new(
        title: &'static str,
        canceled_action: Option<u8>,
        items: Vec<MenuItem>,
        cursor: usize,
    ) -> Self {
        Self {
            title,
            canceled_action,
            items,
            cursor,
            decided: false,
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
            if let MenuItem::SubScene(scene) = self.selected_item() {
                return Some(scene.sub_scene());
            }
            self.decided = true;
            return None;
        }
        let MenuItem::SubMenu(sub_menu) = self.selected_item_mut() else {
            unreachable!()
        };
        sub_menu.sub_menu_mut().dig()
    }

    pub fn bury(&mut self) -> bool {
        if !self.decided {
            return false;
        }
        let MenuItem::SubMenu(sub_menu) = self.selected_item_mut() else {
            self.decided = false;
            return true;
        };
        if !sub_menu.sub_menu().decided {
            self.decided = false;
            return true;
        }
        sub_menu.sub_menu_mut().bury()
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
                self.canceled_action.map(|id| Action::new(id, false, None))
            }
            MenuControllerInputResult::Decide => {
                if ignore_decide {
                    return None;
                }
                if play_decide_sound {
                    th19.play_sound(th19.sound_manager(), 0x07, 0);
                }
                self.selected_item().decided_action()
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
