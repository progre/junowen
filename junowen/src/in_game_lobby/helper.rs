use std::ffi::c_void;

use junowen_lib::{structs::others::RenderingText, Th19};

pub fn render_title(th19: &Th19, text_renderer: *const c_void, text: &[u8]) {
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.set_x(640, th19.window_inner());
    rt.set_y(64, th19.window_inner());
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
    enabled: bool,
    selected: bool,
) {
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.set_x(640, th19.window_inner());
    rt.set_y(y, th19.window_inner());
    rt.color = menu_item_color(9, enabled, selected);
    rt.font_type = 9;
    rt.horizontal_align = 0;
    th19.render_text(text_renderer, &rt);

    rt.color = menu_item_color(7, enabled, selected);
    rt.font_type = 7;
    th19.render_text(text_renderer, &rt);
}

pub fn render_text_line(th19: &Th19, text_renderer: *const c_void, line: u32, text: &[u8]) {
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.set_x(32, th19.window_inner());
    rt.set_y(160 + line * 32, th19.window_inner());
    rt.color = 0xff000000;
    rt.font_type = 8;
    th19.render_text(text_renderer, &rt);

    rt.color = 0xffffffff;
    rt.font_type = 6;
    th19.render_text(text_renderer, &rt);
}

pub fn render_small_text_line(th19: &Th19, text_renderer: *const c_void, line: u32, text: &[u8]) {
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.set_x(32, th19.window_inner());
    rt.set_y(160 + line * 16, th19.window_inner());
    rt.font_type = 1;
    th19.render_text(text_renderer, &rt);
}

pub fn menu_item_color(font_type: u32, enabled: bool, selected: bool) -> u32 {
    if !enabled {
        match font_type {
            9 => 0x40ffffff,
            7 => 0xff808080,
            _ => unreachable!(),
        }
    } else if selected {
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
    let mut rt = RenderingText::default();
    rt.set_text(format!("{:<11}:", label).as_bytes());
    rt.set_x(320, th19.window_inner());
    rt.set_y(height, th19.window_inner());
    rt.color = 0xffffffff;
    rt.font_type = 0;
    rt.horizontal_align = 1;
    rt.vertical_align = vertical_align;
    th19.render_text(text_renderer, &rt);

    rt.set_text(value.as_bytes());
    rt.color = 0xffffffa0;
    rt.set_x(544, th19.window_inner());
    th19.render_text(text_renderer, &rt);
}
