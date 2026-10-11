use std::ffi::c_void;

use junowen_lib::{Th19, structs::others::RenderingText};

use super::overlay::OverlayText;

const TEXT_LINE_LEFT: u32 = 32;
const TEXT_LINE_TOP: u32 = 160;
const TEXT_LINE_HEIGHT: u32 = 32;
const SIGNALING_CODE_CHUNK_LEN: usize = 100;

pub fn render_title(th19: &Th19, text_renderer: &c_void, text: &[u8]) {
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
    text_renderer: &c_void,
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

pub fn render_text_line(th19: &Th19, text_renderer: &c_void, line: u32, text: &[u8]) {
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.set_x(TEXT_LINE_LEFT, th19.window_inner());
    rt.set_y(TEXT_LINE_TOP + line * TEXT_LINE_HEIGHT, th19.window_inner());
    rt.color = 0xff000000;
    rt.font_type = 8;
    th19.render_text(text_renderer, &rt);

    rt.color = 0xffffffff;
    rt.font_type = 6;
    th19.render_text(text_renderer, &rt);
}

/// シグナリングコードは 1 行に収まらないので、半分の行高に詰めて折り返す
pub fn render_signaling_code(th19: &Th19, text_renderer: &c_void, line: u32, code: &str) {
    for (i, chunk) in code.as_bytes().chunks(SIGNALING_CODE_CHUNK_LEN).enumerate() {
        let mut rt = RenderingText::default();
        rt.set_text(chunk);
        rt.set_x(TEXT_LINE_LEFT, th19.window_inner());
        rt.set_y(
            TEXT_LINE_TOP + (line * 2 + i as u32) * (TEXT_LINE_HEIGHT / 2),
            th19.window_inner(),
        );
        rt.font_type = 1;
        th19.render_text(text_renderer, &rt);
    }
}

/// render_signaling_code が消費する行数
pub fn signaling_code_line_count(code: &str) -> u32 {
    let chunks = code.as_bytes().chunks(SIGNALING_CODE_CHUNK_LEN).len();
    (chunks as f64 / 2.0).ceil() as u32
}

/// render_text_line と同じ位置へ多言語テキストを重ねる
pub fn overlay_text_line(line: u32, string: String) -> OverlayText {
    OverlayText {
        left: TEXT_LINE_LEFT as i32,
        top: (TEXT_LINE_TOP + line * TEXT_LINE_HEIGHT) as i32,
        font_size: TEXT_LINE_HEIGHT,
        string,
    }
}

/// 画面下部にメニュー項目の説明文を重ねる
pub fn overlay_description(string: String) -> Vec<OverlayText> {
    if string.is_empty() {
        return vec![];
    }
    vec![OverlayText {
        left: 96,
        top: 640,
        font_size: 58,
        string,
    }]
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
    text_renderer: &c_void,
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
