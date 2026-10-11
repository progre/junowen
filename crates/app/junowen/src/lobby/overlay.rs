use std::collections::HashMap;

use junowen_lib::structs::others::WindowInner;
use windows::Win32::{
    Foundation::{COLORREF, FALSE, RECT, SIZE},
    Graphics::{
        Direct3D9::{D3DSURFACE_DESC, IDirect3DDevice9},
        Gdi::{
            CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_QUALITY, DT_LEFT, DeleteObject, DrawTextW,
            FF_DONTCARE, FW_NORMAL, GetTextExtentPoint32W, HDC, HGDIOBJ, OUT_DEFAULT_PRECIS,
            SHIFTJIS_CHARSET, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, VARIABLE_PITCH,
        },
    },
};
use windows_core::{HSTRING, w};

use super::Lobby;

/// 縁取りの太さと文字サイズの比
const OUTLINE_RATIO: f64 = 1.0 / 29.0;

fn create_font(font_size: u32, scale: f64) -> HGDIOBJ {
    unsafe {
        HGDIOBJ(
            CreateFontW(
                (font_size as f64 * scale) as i32,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                FALSE.0 as u32,
                FALSE.0 as u32,
                FALSE.0 as u32,
                SHIFTJIS_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                DEFAULT_QUALITY,
                (VARIABLE_PITCH.0 + FF_DONTCARE.0) as u32,
                w!("Meiryo"),
            )
            .0,
        )
    }
}

fn render_text_to_screen(
    hdc: HDC,
    surface_desc: D3DSURFACE_DESC,
    font: &Font,
    window_inner: &WindowInner,
    left: i32,
    top: i32,
    string: &str,
) {
    unsafe { SetBkMode(hdc, TRANSPARENT) };

    // GDI で文字を描画
    let h_string = HSTRING::from(string);

    let scale = window_inner.height() as f64 / 960.0;

    let left =
        window_inner.screen_left_offset(surface_desc.Width) as i32 + (left as f64 * scale) as i32;
    let top =
        window_inner.screen_top_offset(surface_desc.Height) as i32 + (top as f64 * scale) as i32;

    let mut size = SIZE::default();
    let mut text = h_string.to_vec();

    let weight = (font.size as f64 * OUTLINE_RATIO * scale).max(1.0) as i32;
    let pos = [
        (-weight, -weight),
        (0, -weight),
        (weight, -weight),
        (-weight, 0),
        (weight, 0),
        (-weight, weight),
        (0, weight),
        (weight, weight),
    ];

    unsafe { SelectObject(hdc, font.obj) };
    unsafe { SetTextColor(hdc, COLORREF(0x000000)) };
    unsafe { GetTextExtentPoint32W(hdc, &h_string, &mut size) }.unwrap();
    for (x, y) in pos {
        let mut rect = RECT {
            left: left + x,
            top: top + y,
            right: left + size.cx,
            bottom: top + size.cy,
        };
        unsafe { DrawTextW(hdc, &mut text, &mut rect, DT_LEFT) };
    }

    unsafe { SelectObject(hdc, font.obj) };
    unsafe { SetTextColor(hdc, COLORREF(0xffffff)) };
    unsafe { GetTextExtentPoint32W(hdc, &h_string, &mut size) }.unwrap();
    let mut rect = RECT {
        left,
        top,
        right: left + size.cx,
        bottom: top + size.cy,
    };
    unsafe { DrawTextW(hdc, &mut text, &mut rect, DT_LEFT) };
}

/// ゲーム内フォントでは英数字しか描画できないため、多言語のテキストは
/// GDI で画面に重ねて描画する。座標はゲーム内と同じ 1280x960 換算。
pub struct OverlayText {
    pub left: i32,
    pub top: i32,
    pub font_size: u32,
    pub string: String,
}

pub struct Font {
    obj: HGDIOBJ,
    size: u32,
}

impl Font {
    pub fn new(size: u32, scale: f64) -> Self {
        Self {
            obj: create_font(size, scale),
            size,
        }
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        if !unsafe { DeleteObject(self.obj) }.as_bool() {
            panic!();
        }
    }
}

pub fn overlay(device: &IDirect3DDevice9, lobby: &Lobby, window_inner: &WindowInner) {
    let texts = lobby.overlay_texts();
    if texts.is_empty() {
        return;
    }

    let surface = unsafe { device.GetRenderTarget(0) }.unwrap();

    let mut hdc = HDC::default();
    unsafe { surface.GetDC(&mut hdc) }.unwrap();

    let mut desc = D3DSURFACE_DESC::default();
    unsafe { surface.GetDesc(&mut desc) }.unwrap();

    let scale = window_inner.height() as f64 / 960.0;
    let mut fonts: HashMap<u32, Font> = HashMap::new();
    for text in &texts {
        fonts
            .entry(text.font_size)
            .or_insert_with(|| Font::new(text.font_size, scale));
    }

    for text in &texts {
        let font = &fonts[&text.font_size];
        for (i, string) in text.string.split('\n').enumerate() {
            render_text_to_screen(
                hdc,
                desc,
                font,
                window_inner,
                text.left,
                text.top + font.size as i32 * i as i32,
                string,
            );
        }
    }

    unsafe { surface.ReleaseDC(hdc) }.unwrap();
}
