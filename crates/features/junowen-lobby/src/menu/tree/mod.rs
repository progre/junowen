mod current_scene;
mod menu_item;

use getset::{CopyGetters, Getters, MutGetters, Setters};

use crate::menu::{
    action::Action,
    device::{MenuDevice, MenuSound},
    input::MenuControllerInputResult,
    render::{MenuRenderer, render_menu_item},
};

pub use current_scene::{
    CurrentMenuSceneMutResult, CurrentMenuSceneResult, current_menu_scene, current_menu_scene_mut,
};
pub use menu_item::MenuItem;

#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
pub struct Menu<S> {
    #[get_copy = "pub"]
    title: &'static str,
    canceled_action: Option<u8>,
    #[getset(get = "pub", get_mut = "pub")]
    items: Vec<MenuItem<S>>,
    #[get_copy = "pub"]
    cursor: usize,
    #[get_copy = "pub"]
    decided: bool,
}

impl<S: Copy> Menu<S> {
    pub fn new(
        title: &'static str,
        canceled_action: Option<u8>,
        items: Vec<MenuItem<S>>,
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

    pub fn selected_item(&self) -> &MenuItem<S> {
        &self.items[self.cursor]
    }

    pub fn selected_item_mut(&mut self) -> &mut MenuItem<S> {
        &mut self.items[self.cursor]
    }

    pub fn dig(&mut self) -> Option<S> {
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

    pub fn on_render_texts(&self, base_height: u32, renderer: &impl MenuRenderer) {
        for (i, item) in self.items().iter().enumerate() {
            let label = item.label().as_bytes();
            let height = base_height + 56 * i as u32;
            render_menu_item(renderer, label, height, item.enabled(), i == self.cursor());
        }
    }

    fn increment_cursor(&mut self) -> bool {
        let Some(new_cursor) = (self.cursor + 1..self.items.len())
            .find(|&new_cursor| self.items[new_cursor].enabled())
        else {
            return false;
        };
        self.cursor = new_cursor;
        true
    }

    fn decrement_cursor(&mut self) -> bool {
        let Some(new_cursor) = (0..self.cursor)
            .rev()
            .find(|&new_cursor| self.items[new_cursor].enabled())
        else {
            return false;
        };
        self.cursor = new_cursor;
        true
    }

    pub fn input(
        &mut self,
        input_result: MenuControllerInputResult,
        ignore_decide: bool,
        play_decide_sound: bool,
        device: &impl MenuDevice,
    ) -> Option<Action> {
        match input_result {
            MenuControllerInputResult::None => None,
            MenuControllerInputResult::Cancel => {
                device.play_sound(MenuSound::Cancel);
                self.canceled_action.map(|id| Action::new(id, false, None))
            }
            MenuControllerInputResult::Decide => {
                if ignore_decide {
                    return None;
                }
                if play_decide_sound {
                    device.play_sound(MenuSound::Decide);
                }
                self.selected_item().decided_action()
            }
            MenuControllerInputResult::Up => {
                if !self.decrement_cursor() {
                    return None;
                }
                device.play_sound(MenuSound::Select);
                None
            }
            MenuControllerInputResult::Down => {
                if !self.increment_cursor() {
                    return None;
                }
                device.play_sound(MenuSound::Select);
                None
            }
        }
    }
}
