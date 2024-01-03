use std::ffi::c_void;

use junowen_lib::{
    structs::{others::RenderingText, settings::GameSettings},
    Th19,
};

pub fn render_names(th19: &Th19, text_renderer: *const c_void, p1_name: &str, p2_name: &str) {
    let mut text = RenderingText::default();
    text.set_text(p1_name.as_bytes());
    text.set_x(16, th19.window_inner());
    text.set_y(4, th19.window_inner());
    text.color = 0xffff8080;
    th19.render_text(text_renderer, &text);

    text.set_text(p2_name.as_bytes());
    text.set_x(1264, th19.window_inner());
    text.color = 0xff8080ff;
    text.horizontal_align = 2;
    th19.render_text(text_renderer, &text);
}

fn render_game_common_settings(
    th19: &Th19,
    text_renderer: *const c_void,
    game_settings: &GameSettings,
) {
    let mut text = RenderingText::default();
    text.set_text(format!("Time Limit: {}", game_settings.time_limit()).as_bytes());
    text.set_x(16, th19.window_inner());
    text.set_y(4 + 32, th19.window_inner());
    text.color = 0xffffffff;
    th19.render_text(text_renderer, &text);

    text.set_text(format!("Round: {}", game_settings.round()).as_bytes());
    text.set_x(1280 - 16, th19.window_inner());
    text.horizontal_align = 2;
    th19.render_text(text_renderer, &text);
}

fn render_game_players_settings(
    th19: &Th19,
    text_renderer: *const c_void,
    game_settings: &GameSettings,
) {
    let mut text = RenderingText::default();

    let y = 870;
    let msg = format!(
        "Life: {}\nBarrier: {}",
        game_settings.p1_life() + 1,
        game_settings.p1_barrier()
    );
    text.set_text(msg.as_bytes());
    text.set_x(16, th19.window_inner());
    text.set_y(y, th19.window_inner());
    text.horizontal_align = 1;
    th19.render_text(text_renderer, &text);

    text.set_text(format!("Life: {}", game_settings.p2_life() + 1,).as_bytes());
    text.set_x(1280 - 16, th19.window_inner());
    text.horizontal_align = 2;
    th19.render_text(text_renderer, &text);

    text.set_text(format!("Barrier: {}", game_settings.p2_barrier()).as_bytes());
    text.set_x(1280 - 16, th19.window_inner());
    text.set_y(y + 28, th19.window_inner());
    th19.render_text(text_renderer, &text);
}

pub fn render_game_settings(
    th19: &Th19,
    text_renderer: *const c_void,
    game_settings: &GameSettings,
) {
    render_game_common_settings(th19, text_renderer, game_settings);
    render_game_players_settings(th19, text_renderer, game_settings);
}

pub fn render_footer(th19: &Th19, text_renderer: *const c_void, msg_front: &str, msg_rear: &str) {
    let version = env!("CARGO_PKG_VERSION");
    let version_blank = (0..version.len()).map(|_| " ").collect::<String>();

    let msg_front/* __ */= format!("Ju.N.Owen v{} {}", version, msg_front);
    let msg_rear/* ___ */= format!("           {} {}", version_blank, msg_rear);

    let mut text = RenderingText::default();
    text.set_text(msg_rear.as_bytes());
    text.set_x(16, th19.window_inner());
    text.set_y(944, th19.window_inner());
    text.color = 0xffffffff;
    text.font_type = 1;
    th19.render_text(text_renderer, &text);

    text.set_text(msg_front.as_bytes());
    text.set_y(940, th19.window_inner());
    th19.render_text(text_renderer, &text);
}
