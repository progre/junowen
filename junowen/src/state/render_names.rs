use std::ffi::c_void;

use junowen_lib::{structs::others::RenderingText, Th19};

pub fn render_names(text_renderer: *const c_void, th19: &Th19, p1_name: &str, p2_name: &str) {
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
