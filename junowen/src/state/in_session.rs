use std::ffi::c_void;

use junowen_lib::RenderingText;

use crate::session::Session;

use super::State;

pub fn on_render_texts(session: &Session, state: &State, text_renderer: *const c_void) {
    let th19 = state.th19();
    let mut text = RenderingText::default();
    text.set_text(
        format!(
            "Ju.N.Owen v{} Delay: {}",
            env!("CARGO_PKG_VERSION"),
            session.delay()
        )
        .as_bytes(),
    );
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (940 * th19.screen_height().unwrap() / 960) as f32;
    text.color = 0xffffffff;
    text.font_type = 1;
    th19.render_text(text_renderer, &text);

    let (p1, p2) = if session.host() {
        (
            th19.player_name().player_name(),
            session.remote_player_name().into(),
        )
    } else {
        (
            session.remote_player_name().into(),
            th19.player_name().player_name(),
        )
    };
    let mut text = RenderingText::default();
    text.set_text(p1.as_bytes());
    text.x = (16 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (4 * th19.screen_height().unwrap() / 1280) as f32;
    text.color = 0xffff8080;
    th19.render_text(text_renderer, &text);

    let mut text = RenderingText::default();
    text.set_text(p2.as_bytes());
    text.x = (1264 * th19.screen_width().unwrap() / 1280) as f32;
    text.y = (4 * th19.screen_height().unwrap() / 1280) as f32;
    text.color = 0xff8080ff;
    text.horizontal_align = 2;
    th19.render_text(text_renderer, &text);
}
