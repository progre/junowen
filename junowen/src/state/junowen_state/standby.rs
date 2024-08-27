use std::ffi::c_void;

use junowen_lib::{
    structs::app::{MainMenu, ScreenId},
    structs::others::RenderingText,
    Fn0b7d40, Fn0d5ae0, Th19,
};

use crate::in_game_lobby::{Lobby, TitleMenuModifier};
use crate::signaling::waiting_for_match::{WaitingForMatch, WaitingForOpponent, WaitingInRoom};

fn is_title(main_menu: &MainMenu) -> bool {
    main_menu.screen_id() == ScreenId::Title
}

fn is_lobby(main_menu: &MainMenu, title_menu_modifier: &TitleMenuModifier) -> bool {
    main_menu.screen_id() == ScreenId::PlayerMatchupSelect && title_menu_modifier.selected_junowen()
}

pub fn update_th19_on_input_menu(
    th19: &mut Th19,
    title_menu_modifier: &mut TitleMenuModifier,
    lobby: &mut Lobby,
) {
    let Some(main_menu) = th19.app_mut().main_loop_tasks_mut().find_main_menu_mut() else {
        return;
    };
    if is_title(main_menu) {
        title_menu_modifier.on_input_menu(main_menu, th19);
    } else if title_menu_modifier.start_lobby(main_menu) {
        lobby.on_input_menu(th19);
    }
}

pub fn render_text(
    th19: &Th19,
    title_menu_modifier: &TitleMenuModifier,
    old: Fn0d5ae0,
    text_renderer: *const c_void,
    text: &mut RenderingText,
) -> u32 {
    let Some(main_menu) = th19.app().main_loop_tasks().find_main_menu() else {
        return old(text_renderer, text);
    };
    title_menu_modifier.render_text(main_menu, th19, old, text_renderer, text)
}

fn render_message(text_renderer: *const c_void, th19: &Th19, msg: &str, color: u32) {
    let mut text = RenderingText::default();
    text.set_text(msg.as_bytes());
    text.set_x(16, th19.window_inner());
    text.set_y(4, th19.window_inner());
    text.color = color;
    th19.render_text(text_renderer, &text);
}

fn render_waiting_message<T>(
    room_type: &str,
    room: &WaitingInRoom<T>,
    th19: &Th19,
    text_renderer: *const c_void,
) {
    let room_name = room.room_name();
    let dot = ".".repeat((room.elapsed().as_secs() % 4) as usize);
    let msg = format!("Waiting in {} Room: {} {:<3}", room_type, room_name, dot);
    render_message(text_renderer, th19, &msg, 0xffc0c0c0);
    if !room.errors().is_empty() {
        let padding = " ".repeat(msg.chars().count());
        let msg = format!("{} E({})", padding, room.errors().len());
        render_message(text_renderer, th19, &msg, 0xffff2800);
    }
}

pub fn on_render_texts(
    th19: &Th19,
    title_menu_modifier: &TitleMenuModifier,
    lobby: &Lobby,
    text_renderer: *const c_void,
) {
    match lobby.waiting_for_match() {
        None
        | Some(WaitingForMatch::SpectatorHost(_))
        | Some(WaitingForMatch::Opponent(WaitingForOpponent::PureP2p(_))) => {}
        Some(WaitingForMatch::Opponent(WaitingForOpponent::SharedRoom(waiting))) => {
            render_waiting_message("Shared", waiting, th19, text_renderer);
        }
        Some(WaitingForMatch::Opponent(WaitingForOpponent::ReservedRoom(waiting))) => {
            render_waiting_message("Reserved", waiting, th19, text_renderer);
        }
    }
    let Some(main_menu) = th19.app().main_loop_tasks().find_main_menu() else {
        return;
    };
    if is_lobby(main_menu, title_menu_modifier) {
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
        if (0xb4..=0xc0).contains(&id) {
            return;
        }
    }
    old(obj_renderer, obj);
}
