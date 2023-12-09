use std::ffi::c_void;

use junowen_lib::{RenderingText, Th19};

pub fn render_title(th19: &Th19, text_renderer: *const c_void, text: &[u8]) {
    let x = (640 * th19.screen_width().unwrap() / 1280) as f32;
    let y = (64 * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.x = x;
    rt.y = y;
    rt.color = 0xff000000;
    rt.font_type = 9;
    rt.drop_shadow = true;
    rt.horizontal_align = 0;
    th19.render_text(text_renderer, &rt);

    rt.color = 0xffffffff;
    rt.font_type = 7;
    th19.render_text(text_renderer, &rt);
}

pub fn render_menu_item(
    th19: &Th19,
    text_renderer: *const c_void,
    text: &[u8],
    y: u32,
    selected: bool,
) {
    let x = (640 * th19.screen_width().unwrap() / 1280) as f32;
    let y = (y * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.x = x;
    rt.y = y;
    rt.color = menu_item_color(9, selected);
    rt.font_type = 9;
    rt.horizontal_align = 0;
    th19.render_text(text_renderer, &rt);

    rt.color = menu_item_color(7, selected);
    rt.font_type = 7;
    th19.render_text(text_renderer, &rt);
}

pub fn render_text_line(th19: &Th19, text_renderer: *const c_void, line: u32, text: &[u8]) {
    let x = (32 * th19.screen_width().unwrap() / 1280) as f32;
    let y = ((160 + line * 32) * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.x = x;
    rt.y = y;
    rt.color = 0xff000000;
    rt.font_type = 8;
    th19.render_text(text_renderer, &rt);

    rt.color = 0xffffffff;
    rt.font_type = 6;
    th19.render_text(text_renderer, &rt);
}

pub fn render_small_text_line(th19: &Th19, text_renderer: *const c_void, line: u32, text: &[u8]) {
    let x = (32 * th19.screen_width().unwrap() / 1280) as f32;
    let y = ((160 + line * 16) * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.x = x;
    rt.y = y;
    rt.font_type = 1;
    th19.render_text(text_renderer, &rt);
}

pub fn menu_item_color(font_type: u32, selected: bool) -> u32 {
    if selected {
        match font_type {
            9 => 0xff000000,
            7 => 0xffffff80,
            _ => unreachable!(),
        }
    } else {
        match font_type {
            9 => 0xff404040,
            7 => 0xff808060,
            _ => unreachable!(),
        }
    }
}

pub fn render_label_value(
    th19: &Th19,
    text_renderer: *const c_void,
    height: u32,
    vertical_align: u32,
    label: &str,
    value: &str,
) {
    let x = (320 * th19.screen_width().unwrap() / 1280) as f32;
    let y = (height * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text(format!("{:<11}:", label).as_bytes());
    rt.x = x;
    rt.y = y;
    rt.color = 0xffffffff;
    rt.font_type = 0;
    rt.horizontal_align = 1;
    rt.vertical_align = vertical_align;
    th19.render_text(text_renderer, &rt);

    let x = (544 * th19.screen_width().unwrap() / 1280) as f32;
    rt.set_text(value.as_bytes());
    rt.color = 0xffffffa0;
    rt.x = x;
    th19.render_text(text_renderer, &rt);
}
