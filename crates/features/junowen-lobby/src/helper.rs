use crate::{MenuRenderer, MenuText, OverlayText};

const TEXT_LINE_LEFT: u32 = 32;
const TEXT_LINE_TOP: u32 = 160;
const TEXT_LINE_HEIGHT: u32 = 32;
const SIGNALING_CODE_CHUNK_LEN: usize = 100;

pub fn render_text_line(renderer: &impl MenuRenderer, line: u32, text: &[u8]) {
    let mut mt = MenuText {
        text,
        x: TEXT_LINE_LEFT,
        y: TEXT_LINE_TOP + line * TEXT_LINE_HEIGHT,
        color: 0xff000000,
        font_type: 8,
        ..Default::default()
    };
    renderer.render_text(&mt);

    mt.color = 0xffffffff;
    mt.font_type = 6;
    renderer.render_text(&mt);
}

/// シグナリングコードは 1 行に収まらないので、半分の行高に詰めて折り返す
pub fn render_signaling_code(renderer: &impl MenuRenderer, line: u32, code: &str) {
    for (i, chunk) in code.as_bytes().chunks(SIGNALING_CODE_CHUNK_LEN).enumerate() {
        renderer.render_text(&MenuText {
            text: chunk,
            x: TEXT_LINE_LEFT,
            y: TEXT_LINE_TOP + (line * 2 + i as u32) * (TEXT_LINE_HEIGHT / 2),
            font_type: 1,
            ..Default::default()
        });
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
