use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;

pub trait Direct3DDeviceEventListener {
    fn on_before_present(&self, device: &IDirect3DDevice9);
}
