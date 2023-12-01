use derive_new::new;
use getset::{CopyGetters, Getters, MutGetters, Setters};

use super::LobbyScene;

#[derive(Clone, Debug, CopyGetters, new)]
pub struct Action {
    #[get_copy = "pub"]
    id: u8,
    #[get_copy = "pub"]
    play_sound: bool,
}

#[derive(Debug)]
pub enum MenuChild {
    SubMenu(MenuDefine),
    SubScene(LobbyScene),
}

#[derive(Debug, CopyGetters, Getters, MutGetters)]
pub struct MenuItem {
    #[get_copy = "pub"]
    label: &'static str,
    action: Option<Action>,
    #[getset(get = "pub", get_mut = "pub")]
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
        Self::new(label, Some(Action::new(id, play_sound)), None)
    }

    pub fn simple_sub_scene(label: &'static str, scene: LobbyScene) -> Self {
        Self::new(label, None, Some(MenuChild::SubScene(scene)))
    }

    pub fn action(&self) -> Option<&Action> {
        self.action.as_ref()
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
