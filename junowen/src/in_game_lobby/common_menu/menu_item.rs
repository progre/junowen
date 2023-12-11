use getset::{CopyGetters, Getters, MutGetters};

use super::{menu::Menu, text_input::TextInput, Action, LobbyScene};

#[derive(Debug)]
pub struct MenuPlainItem {
    label: &'static str,
    // enabled: bool,
    decided_action: u8,
    play_sound: bool,
}

#[derive(Debug, Getters, MutGetters)]
pub struct MenuSubMenuItem {
    label: &'static str,
    // enabled: bool,
    // wait: bool,
    decided_action: Option<u8>,
    #[getset(get = "pub", get_mut = "pub")]
    sub_menu: Menu,
}

#[derive(CopyGetters, Debug, Getters, MutGetters)]
pub struct MenuTextInputItem {
    #[get_copy = "pub"]
    label: &'static str,
    // enabled: bool,
    decided_action: u8,
    #[getset(get = "pub", get_mut = "pub")]
    text_input: Box<TextInput>,
}

#[derive(CopyGetters, Debug)]
pub struct MenuSubSceneItem {
    #[get_copy = "pub"]
    label: &'static str,
    // enabled: bool,
    #[get_copy = "pub"]
    sub_scene: LobbyScene,
}

#[derive(Debug)]
pub enum MenuItem {
    Plain(MenuPlainItem),
    SubMenu(MenuSubMenuItem),
    TextInput(MenuTextInputItem),
    SubScene(MenuSubSceneItem),
}

impl MenuItem {
    pub fn plain(label: &'static str, decided_action: u8, play_sound: bool) -> Self {
        Self::Plain(MenuPlainItem {
            label,
            decided_action,
            play_sound,
        })
    }

    pub fn sub_menu(label: &'static str, decided_action: Option<u8>, sub_menu: Menu) -> Self {
        Self::SubMenu(MenuSubMenuItem {
            label,
            decided_action,
            sub_menu,
        })
    }

    pub fn sub_scene(label: &'static str, sub_scene: LobbyScene) -> Self {
        Self::SubScene(MenuSubSceneItem { label, sub_scene })
    }

    pub fn text_input(
        label: &'static str,
        decided_action: u8,
        changed_action: u8,
        name: &'static str,
    ) -> Self {
        Self::TextInput(MenuTextInputItem {
            label,
            decided_action,
            text_input: Box::new(TextInput::new(changed_action, name)),
        })
    }

    // ----

    pub fn label(&self) -> &str {
        match self {
            Self::Plain(item) => item.label,
            Self::SubMenu(item) => item.label,
            Self::TextInput(item) => item.label,
            Self::SubScene(scene) => scene.label,
        }
    }

    pub fn decided_action(&self) -> Option<Action> {
        match self {
            Self::Plain(item) => Some(Action::new(item.decided_action, item.play_sound, None)),
            Self::SubMenu(item) => item
                .decided_action
                .map(|decided_action| Action::new(decided_action, true, None)),
            Self::TextInput(item) => Some(Action::new(
                item.decided_action,
                true,
                Some(item.text_input().value().to_owned()),
            )),
            Self::SubScene(_) => None,
        }
    }
}
