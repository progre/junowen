use std::ffi::c_void;

use junowen_lib::{structs::others::RenderingText, Th19};

pub fn render_names(th19: &Th19, text_renderer: *const c_void, p1_name: &str, p2_name: &str) {
    let mut text = RenderingText::default();
    text.set_text(p1_name.as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (4 * th19.screen_height().unwrap() / 1280) as f32;
    text.color = 0xffff8080;
    th19.render_text(text_renderer, &text);

    text.set_text(p2_name.as_bytes());
    text.x = (1264 * th19.screen_width().unwrap() / 1280) as f32;
    text.color = 0xff8080ff;
    text.horizontal_align = 2;
    th19.render_text(text_renderer, &text);
}

pub fn render_footer(th19: &Th19, text_renderer: *const c_void, msg_front: &str, msg_rear: &str) {
    let version = env!("CARGO_PKG_VERSION");
    let version_blank = (0..version.len()).map(|_| " ").collect::<String>();

    let msg_front/* __ */= format!("Ju.N.Owen v{} {}", version, msg_front);
    let msg_rear/* ___ */= format!("           {} {}", version_blank, msg_rear);

    let mut text = RenderingText::default();
    text.set_text(msg_rear.as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (944 * th19.screen_height().unwrap() / 960) as f32;
    text.color = 0xffffffff;
    text.font_type = 1;
    th19.render_text(text_renderer, &text);

    text.set_text(msg_front.as_bytes());
    text.y = (940 * th19.screen_height().unwrap() / 960) as f32;
    th19.render_text(text_renderer, &text);
}
