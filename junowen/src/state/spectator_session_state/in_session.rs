use std::ffi::c_void;

use junowen_lib::Th19;

use crate::state::render_parts::{render_footer, render_names};

pub fn on_render_texts_spectator(
    th19: &Th19,
    text_renderer: *const c_void,
    p1_name: &str,
    p2_name: &str,
) {
    render_names(th19, text_renderer, p1_name, p2_name);
    render_footer(th19, text_renderer, "(Spectating)", "");
}
