use std::ffi::c_void;

use junowen_lib::Th19;

use super::{
    helper::{overlay_text_line, render_signaling_code},
    overlay::OverlayText,
};

/// 画面に並ぶ要素を行番号で組み立てたもの。
/// テキストは多言語を表示するため overlay で、シグナリングコードは
/// 英数字だけなのでゲーム内フォントで描画する。
#[derive(Default)]
pub struct TextLayout {
    texts: Vec<(u32, String)>,
    codes: Vec<(u32, String)>,
}

impl TextLayout {
    pub fn push_text(&mut self, line: u32, text: impl Into<String>) {
        self.texts.push((line, text.into()));
    }

    pub fn push_code(&mut self, line: u32, code: impl Into<String>) {
        self.codes.push((line, code.into()));
    }

    pub fn render_codes(&self, th19: &Th19, text_renderer: &c_void) {
        for (line, code) in &self.codes {
            render_signaling_code(th19, text_renderer, *line, code);
        }
    }

    pub fn into_overlay_texts(self) -> Vec<OverlayText> {
        self.texts
            .into_iter()
            .map(|(line, text)| overlay_text_line(line, text))
            .collect()
    }
}
