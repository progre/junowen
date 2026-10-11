/// ゲーム内フォントで描画するテキスト。座標はゲーム内と同じ 1280x960 換算
#[derive(Clone, Copy, Debug)]
pub struct MenuText<'a> {
    pub text: &'a [u8],
    pub x: u32,
    pub y: u32,
    /// 0xaarrggbb
    pub color: u32,
    pub font_type: u32,
    pub drop_shadow: bool,
    /// 0: center, 1: left, 2: right
    pub horizontal_align: u32,
    /// 0: center, 1: top, 2: bottom
    pub vertical_align: u32,
}

impl Default for MenuText<'_> {
    fn default() -> Self {
        Self {
            text: b"",
            x: 0,
            y: 0,
            color: 0xffffffff,
            font_type: 0,
            drop_shadow: false,
            horizontal_align: 1,
            vertical_align: 1,
        }
    }
}

pub trait MenuRenderer {
    fn render_text(&self, text: &MenuText);
}

pub fn render_title(renderer: &impl MenuRenderer, text: &[u8]) {
    let mut mt = MenuText {
        text,
        x: 640,
        y: 64,
        color: 0xff000000,
        font_type: 9,
        drop_shadow: true,
        horizontal_align: 0,
        ..Default::default()
    };
    renderer.render_text(&mt);

    mt.color = 0xffffffff;
    mt.font_type = 7;
    renderer.render_text(&mt);
}

pub fn render_menu_item(
    renderer: &impl MenuRenderer,
    text: &[u8],
    y: u32,
    enabled: bool,
    selected: bool,
) {
    let mut mt = MenuText {
        text,
        x: 640,
        y,
        color: menu_item_color(9, enabled, selected),
        font_type: 9,
        horizontal_align: 0,
        ..Default::default()
    };
    renderer.render_text(&mt);

    mt.color = menu_item_color(7, enabled, selected);
    mt.font_type = 7;
    renderer.render_text(&mt);
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
    renderer: &impl MenuRenderer,
    height: u32,
    vertical_align: u32,
    label: &str,
    value: &str,
) {
    let label = format!("{:<11}:", label);
    let mut mt = MenuText {
        text: label.as_bytes(),
        x: 320,
        y: height,
        color: 0xffffffff,
        font_type: 0,
        horizontal_align: 1,
        vertical_align,
        ..Default::default()
    };
    renderer.render_text(&mt);

    mt.text = value.as_bytes();
    mt.color = 0xffffffa0;
    mt.x = 544;
    renderer.render_text(&mt);
}
