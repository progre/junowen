#[derive(Clone, Copy, Debug)]
pub enum MenuSound {
    Cancel,
    Decide,
    Select,
}

/// メニュー操作に使うゲームの機能
pub trait MenuDevice {
    fn play_sound(&self, sound: MenuSound);
    /// 仮想キーコードごとの押下状態。最上位ビットが立っていれば押されている
    fn keyboard_raw_keys(&self) -> &[u8; 256];
}
