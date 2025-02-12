use getset::{CopyGetters, Getters, MutGetters, Setters};

use super::{menu::Menu, text_input::TextInput, Action, LobbyScene};

#[derive(Debug, Setters)]
pub struct MenuPlainItem {
    #[set = "pub"]
    label: &'static str,
    enabled: bool,
    decided_action: u8,
    play_sound: bool,
}

#[derive(Debug, Getters, MutGetters, Setters)]
pub struct MenuSubMenuItem {
    #[set = "pub"]
    label: &'static str,
    enabled: bool,
    decided_action: Option<u8>,
    #[getset(get = "pub", get_mut = "pub")]
    sub_menu: Menu,
}

#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
pub struct MenuTextInputItem {
    #[getset(get_copy = "pub", set = "pub")]
    label: &'static str,
    enabled: bool,
    decided_action: u8,
    #[getset(get = "pub", get_mut = "pub")]
    text_input: Box<TextInput>,
}

#[derive(CopyGetters, Debug, Setters)]
pub struct MenuSubSceneItem {
    #[getset(get_copy = "pub", set = "pub")]
    label: &'static str,
    enabled: bool,
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
            enabled: true,
            decided_action,
            play_sound,
        })
    }

    pub fn sub_menu(label: &'static str, decided_action: Option<u8>, sub_menu: Menu) -> Self {
        Self::SubMenu(MenuSubMenuItem {
            label,
            enabled: true,
            decided_action,
            sub_menu,
        })
    }

    pub fn sub_scene(label: &'static str, sub_scene: LobbyScene) -> Self {
        Self::SubScene(MenuSubSceneItem {
            label,
            enabled: true,
            sub_scene,
        })
    }

    pub fn text_input(
        label: &'static str,
        decided_action: u8,
        changed_action: u8,
        name: &'static str,
    ) -> Self {
        Self::TextInput(MenuTextInputItem {
            label,
            enabled: true,
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
    pub fn set_label(&mut self, label: &'static str) {
        match self {
            Self::Plain(item) => item.label = label,
            Self::SubMenu(item) => item.label = label,
            Self::TextInput(item) => item.label = label,
            Self::SubScene(scene) => scene.label = label,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Plain(item) => item.enabled,
            Self::SubMenu(item) => item.enabled,
            Self::TextInput(item) => item.enabled,
            Self::SubScene(scene) => scene.enabled,
        }
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        match self {
            Self::Plain(item) => item.enabled = enabled,
            Self::SubMenu(item) => item.enabled = enabled,
            Self::TextInput(item) => item.enabled = enabled,
            Self::SubScene(scene) => scene.enabled = enabled,
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
