use std::ffi::c_void;

use junowen_lib::{Fn0b7d40, Fn0d5ae0, Menu, ScreenId};

use crate::in_game_lobby::TitleMenuModifier;

use super::State;

fn is_title(menu: &Menu) -> bool {
    menu.screen_id == ScreenId::Title
}

fn is_lobby(menu: &Menu, title_menu_modifier: &TitleMenuModifier) -> bool {
    menu.screen_id == ScreenId::PlayerMatchupSelect && title_menu_modifier.selected_junowen()
}

pub fn on_input_menu(state: &mut State) {
    let th19 = &mut state.th19;
    let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
        return;
    };
    let title_menu_modifier = &mut state.title_menu_modifier;
    if is_title(menu) {
        title_menu_modifier.on_input_menu(menu, th19);
    } else if title_menu_modifier.start_lobby(menu) {
        state.lobby.on_input_menu(th19);
    }
}

pub fn render_text(
    state: &mut State,
    old: Fn0d5ae0,
    text_renderer: *const c_void,
    text: &mut junowen_lib::RenderingText,
) -> u32 {
    let th19 = &mut state.th19;
    let Some(menu) = th19.app().main_loop_tasks().find_menu() else {
        return old(text_renderer, text);
    };
    let title_menu_modifier = &state.title_menu_modifier;
    title_menu_modifier.render_text(menu, th19, old, text_renderer, text)
}

pub fn on_render_texts(state: &mut State, text_renderer: *const c_void) {
    let th19 = &mut state.th19;
    let Some(menu) = th19.app().main_loop_tasks().find_menu() else {
        return;
    };
    if is_lobby(menu, &state.title_menu_modifier) {
        state.lobby.on_render_texts(th19, text_renderer);
    }
}

pub fn render_object(
    state: &State,
    old: Fn0b7d40,
    obj_renderer: *const c_void,
    obj: *const c_void,
) {
    if state.title_menu_modifier.selected_junowen() {
        let id = unsafe { *(obj.add(0x28) as *const u32) };
        if (0xb0..=0xbc).contains(&id) {
            return;
        }
    }
    old(obj_renderer, obj);
}
