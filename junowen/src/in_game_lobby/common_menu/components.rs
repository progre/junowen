use derive_new::new;
use getset::{CopyGetters, Getters, MutGetters, Setters};

use super::LobbyScene;

#[derive(Debug)]
pub enum MenuAction {
    Action(u8, bool),
    SubScene(LobbyScene),
}

#[derive(Debug)]
pub enum MenuContent {
    Action(MenuAction),
    SubMenu(MenuDefine),
}

impl From<MenuAction> for MenuContent {
    fn from(value: MenuAction) -> Self {
        MenuContent::Action(value)
    }
}

#[derive(Debug, CopyGetters, Getters, MutGetters)]
pub struct MenuItem {
    #[get_copy = "pub"]
    label: &'static str,
    #[getset(get = "pub", get_mut = "pub")]
    content: MenuContent,
}

impl MenuItem {
    pub fn new(label: &'static str, content: MenuContent) -> Self {
        Self { label, content }
    }
}

#[derive(Debug, CopyGetters, Getters, Setters, new)]
pub struct MenuDefine {
    #[getset(get_copy = "pub", set = "pub")]
    cursor: usize,
    #[get = "pub"]
    items: Vec<MenuItem>,
}

impl MenuDefine {
    pub fn selected_item(&self) -> &MenuItem {
        &self.items[self.cursor]
    }

    pub fn selected_item_mut(&mut self) -> &mut MenuItem {
        &mut self.items[self.cursor]
    }
}
