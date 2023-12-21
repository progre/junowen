use std::ffi::c_void;

use junowen_lib::{structs::others::RenderingText, Th19};

use crate::state::render_names::render_names;

pub fn on_render_texts_spectator(
    th19: &Th19,
    p1_name: &str,
    p2_name: &str,
    text_renderer: *const c_void,
) {
    let msg2 = "(Spectating)";
    let mut text = RenderingText::default();
    text.set_text(format!("Ju.N.Owen v{} {}", env!("CARGO_PKG_VERSION"), msg2).as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (940 * th19.screen_height().unwrap() / 960) as f32;
    text.color = 0xffffffff;
    text.font_type = 1;
    th19.render_text(text_renderer, &text);

    render_names(text_renderer, th19, p1_name, p2_name);
}
