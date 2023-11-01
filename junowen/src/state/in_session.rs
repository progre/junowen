use std::ffi::c_void;

use junowen_lib::{Fn10f720, RenderingText, Th19};
use tracing::trace;

fn render_names(text_renderer: *const c_void, th19: &Th19, p1_name: &str, p2_name: &str) {
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

pub fn on_render_texts(
    th19: &Th19,
    host: bool,
    delay: u8,
    p1_name: &str,
    p2_name: &str,
    text_renderer: *const c_void,
) {
    let version = env!("CARGO_PKG_VERSION");

    let version_blank = (0..version.len()).map(|_| " ").collect::<String>();
    let delay_underline = if host { "_" } else { " " };
    let mut text = RenderingText::default();
    text.set_text(format!("           {}        {}", version_blank, delay_underline).as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (944 * th19.screen_height().unwrap() / 960) as f32;
    text.color = 0xffffffff;
    text.font_type = 1;
    th19.render_text(text_renderer, &text);

    text.set_text(format!("Ju.N.Owen v{} Delay: {}", version, delay).as_bytes());
    text.y = (940 * th19.screen_height().unwrap() / 960) as f32;
    th19.render_text(text_renderer, &text);

    render_names(text_renderer, th19, p1_name, p2_name);
}

pub fn on_rewrite_controller_assignments(th19: &mut Th19, old_fn: fn(&mut Th19) -> Fn10f720) {
    let input_devices = th19.input_devices_mut();
    let old_p1_idx = input_devices.p1_idx();
    trace!(
        "on_rewrite_controller_assignments: before old_p1_idx={}",
        old_p1_idx
    );
    old_fn(th19)();
    if old_p1_idx == 0 && input_devices.p1_idx() != 0 {
        trace!(
            "on_rewrite_controller_assignments: after input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
        input_devices.set_p1_idx(0);
        trace!(
            "on_rewrite_controller_assignments: fixed input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
    }
}
