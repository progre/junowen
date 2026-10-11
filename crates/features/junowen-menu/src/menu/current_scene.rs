use crate::input::TextInput;

use super::{Menu, MenuItem};

#[derive(Debug)]
pub enum CurrentMenuSceneResult<'a, S> {
    Menu(&'a Menu<S>),
    SubScene(S),
    TextInput(&'static str, &'a TextInput),
}

pub enum CurrentMenuSceneMutResult<'a, S> {
    Menu(&'a mut Menu<S>),
    SubScene,
    TextInput(&'a mut TextInput),
}

/// 決定済みのサブメニューをたどり、操作対象の画面を返す
pub fn current_menu_scene<S: Copy>(root: &Menu<S>) -> CurrentMenuSceneResult<'_, S> {
    if !root.decided() {
        return CurrentMenuSceneResult::Menu(root);
    }
    let mut menu = root;
    loop {
        match menu.selected_item() {
            MenuItem::Plain(_) => unreachable!(),
            MenuItem::SubMenu(item) => {
                if item.sub_menu().decided() {
                    menu = item.sub_menu();
                    continue;
                }
                return CurrentMenuSceneResult::Menu(item.sub_menu());
            }
            MenuItem::SubScene(item) => {
                return CurrentMenuSceneResult::SubScene(item.sub_scene());
            }
            MenuItem::TextInput(item) => {
                return CurrentMenuSceneResult::TextInput(item.label(), item.text_input());
            }
        }
    }
}

pub fn current_menu_scene_mut<S: Copy>(root: &mut Menu<S>) -> CurrentMenuSceneMutResult<'_, S> {
    if !root.decided() {
        return CurrentMenuSceneMutResult::Menu(root);
    }
    let mut menu = root;
    loop {
        match menu.selected_item_mut() {
            MenuItem::Plain(_) => unreachable!(),
            MenuItem::SubMenu(item) => {
                if item.sub_menu().decided() {
                    menu = item.sub_menu_mut();
                    continue;
                }
                return CurrentMenuSceneMutResult::Menu(item.sub_menu_mut());
            }
            MenuItem::SubScene(_item) => {
                return CurrentMenuSceneMutResult::SubScene;
            }
            MenuItem::TextInput(item) => {
                return CurrentMenuSceneMutResult::TextInput(item.text_input_mut());
            }
        }
    }
}
