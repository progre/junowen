/// ゲーム内フォントでは英数字しか描画できないため、多言語のテキストは
/// GDI で画面に重ねて描画する。座標はゲーム内と同じ 1280x960 換算。
pub struct OverlayText {
    pub left: i32,
    pub top: i32,
    pub font_size: u32,
    pub string: String,
}
