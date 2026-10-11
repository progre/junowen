use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;

pub trait Direct3DDeviceEventListener {
    /// 表示を間引く場合は `true` を返す
    fn should_skip_present(&self) -> bool;
    fn on_before_present(&self, device: &IDirect3DDevice9);
}
