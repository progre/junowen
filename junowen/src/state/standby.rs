use std::ffi::c_void;

use junowen_lib::{Fn0b7d40, Fn0d5ae0, Menu, RenderingText, ScreenId, Th19};

use crate::in_game_lobby::{Lobby, TitleMenuModifier, WaitingForMatch, WaitingForOpponent};

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

fn render_message(text_renderer: *const c_void, th19: &Th19, msg: &str, color: u32) {
    let mut text = RenderingText::default();
    text.set_text(msg.as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (4 * th19.screen_height().unwrap() / 1280) as f32;
    text.color = color;
    th19.render_text(text_renderer, &text);
}

pub fn on_render_texts(
    th19: &Th19,
    title_menu_modifier: &TitleMenuModifier,
    lobby: &Lobby,
    text_renderer: *const c_void,
) {
    match lobby.waiting_for_match() {
        None
        | Some(WaitingForMatch::Spectator(_))
        | Some(WaitingForMatch::Opponent(WaitingForOpponent::PureP2p(_))) => {}
        Some(WaitingForMatch::Opponent(WaitingForOpponent::SharedRoom(waiting))) => {
            let msg = format!(
                "Waiting in Shared Room: {} {:<3}",
                waiting.room_name(),
                ".".repeat((waiting.elapsed().as_secs() % 4) as usize)
            );
            render_message(text_renderer, th19, &msg, 0xffc0c0c0);
            if !waiting.errors().is_empty() {
                let msg = format!(
                    "{} E({})",
                    " ".repeat(msg.chars().count()),
                    waiting.errors().len()
                );
                render_message(text_renderer, th19, &msg, 0xffff2800);
            }
        }
    }
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
