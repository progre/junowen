use std::ffi::c_void;

use junowen_lib::{Fn0b7d40, Fn0d5ae0, Menu, ScreenId, Th19};

use crate::in_game_lobby::{Lobby, TitleMenuModifier};

fn is_title(menu: &Menu) -> bool {
    menu.screen_id == ScreenId::Title
}

fn is_lobby(menu: &Menu, title_menu_modifier: &TitleMenuModifier) -> bool {
    menu.screen_id == ScreenId::PlayerMatchupSelect && title_menu_modifier.selected_junowen()
}

pub fn update_th19_on_input_menu(
    th19: &mut Th19,
    title_menu_modifier: &mut TitleMenuModifier,
    lobby: &mut Lobby,
) {
    let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
        return;
    };
    if is_title(menu) {
        title_menu_modifier.on_input_menu(menu, th19);
    } else if title_menu_modifier.start_lobby(menu) {
        lobby.on_input_menu(th19);
    }
}

pub fn render_text(
    th19: &Th19,
    title_menu_modifier: &TitleMenuModifier,
    old: Fn0d5ae0,
    text_renderer: *const c_void,
    text: &mut junowen_lib::RenderingText,
) -> u32 {
    let Some(menu) = th19.app().main_loop_tasks().find_menu() else {
        return old(text_renderer, text);
    };
    title_menu_modifier.render_text(menu, th19, old, text_renderer, text)
}

pub fn on_render_texts(
    th19: &Th19,
    title_menu_modifier: &TitleMenuModifier,
    lobby: &Lobby,
    text_renderer: *const c_void,
) {
    let Some(menu) = th19.app().main_loop_tasks().find_menu() else {
        return;
    };
    if is_lobby(menu, title_menu_modifier) {
        lobby.on_render_texts(th19, text_renderer);
    }
}

pub fn render_object(
    title_menu_modifier: &TitleMenuModifier,
    old: Fn0b7d40,
    obj_renderer: *const c_void,
    obj: *const c_void,
) {
    if title_menu_modifier.selected_junowen() {
        let id = unsafe { *(obj.add(0x28) as *const u32) };
        if (0xb0..=0xbc).contains(&id) {
            return;
        }
    }
    old(obj_renderer, obj);
}
