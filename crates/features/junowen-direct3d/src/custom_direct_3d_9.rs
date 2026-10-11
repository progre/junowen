use std::ffi::c_void;

use windows::{
    Win32::{
        Foundation::{BOOL, HWND},
        Graphics::{
            Direct3D9::{
                D3DADAPTER_IDENTIFIER9, D3DCAPS9, D3DDEVTYPE, D3DDISPLAYMODE, D3DFORMAT,
                D3DMULTISAMPLE_TYPE, D3DPRESENT_PARAMETERS, D3DRESOURCETYPE, IDirect3D9,
                IDirect3D9_Impl, IDirect3DDevice9,
            },
            Gdi::HMONITOR,
        },
    },
    core::{OutRef, implement},
};

use super::{Direct3DDeviceEventListener, custom_direct_3d_device_9::CustomDirect3DDevice9};

#[implement(IDirect3D9)]
pub struct CustomDirect3D9<T>
where
    T: 'static + Direct3DDeviceEventListener + Clone,
{
    instance: IDirect3D9,
    delegate: T,
}

impl<T> CustomDirect3D9<T>
where
    T: 'static + Direct3DDeviceEventListener + Clone,
{
    pub fn new(instance: IDirect3D9, delegate: T) -> Self {
        Self { instance, delegate }
    }
}

impl<T> IDirect3D9_Impl for CustomDirect3D9_Impl<T>
where
    T: Direct3DDeviceEventListener + Clone,
{
    fn RegisterSoftwareDevice(
        &self,
        pinitializefunction: *mut c_void,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.RegisterSoftwareDevice(pinitializefunction) }
    }

    fn GetAdapterCount(&self) -> u32 {
        unsafe { self.instance.GetAdapterCount() }
    }

    fn GetAdapterIdentifier(
        &self,
        adapter: u32,
        flags: u32,
        pidentifier: *mut D3DADAPTER_IDENTIFIER9,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetAdapterIdentifier(adapter, flags, pidentifier)
        }
    }

    fn GetAdapterModeCount(&self, adapter: u32, format: D3DFORMAT) -> u32 {
        unsafe { self.instance.GetAdapterModeCount(adapter, format) }
    }

    fn EnumAdapterModes(
        &self,
        adapter: u32,
        format: D3DFORMAT,
        mode: u32,
        pmode: *mut D3DDISPLAYMODE,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.EnumAdapterModes(adapter, format, mode, pmode) }
    }

    fn GetAdapterDisplayMode(
        &self,
        adapter: u32,
        pmode: *mut D3DDISPLAYMODE,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetAdapterDisplayMode(adapter, pmode) }
    }

    fn CheckDeviceType(
        &self,
        adapter: u32,
        devtype: D3DDEVTYPE,
        adapterformat: D3DFORMAT,
        backbufferformat: D3DFORMAT,
        bwindowed: BOOL,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.CheckDeviceType(
                adapter,
                devtype,
                adapterformat,
                backbufferformat,
                bwindowed.as_bool(),
            )
        }
    }

    fn CheckDeviceFormat(
        &self,
        adapter: u32,
        devicetype: D3DDEVTYPE,
        adapterformat: D3DFORMAT,
        usage: u32,
        rtype: D3DRESOURCETYPE,
        checkformat: D3DFORMAT,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.CheckDeviceFormat(
                adapter,
                devicetype,
                adapterformat,
                usage,
                rtype,
                checkformat,
            )
        }
    }

    fn CheckDeviceMultiSampleType(
        &self,
        adapter: u32,
        devicetype: D3DDEVTYPE,
        surfaceformat: D3DFORMAT,
        windowed: BOOL,
        multisampletype: D3DMULTISAMPLE_TYPE,
        pqualitylevels: *mut u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.CheckDeviceMultiSampleType(
                adapter,
                devicetype,
                surfaceformat,
                windowed.as_bool(),
                multisampletype,
                pqualitylevels,
            )
        }
    }

    fn CheckDepthStencilMatch(
        &self,
        adapter: u32,
        devicetype: D3DDEVTYPE,
        adapterformat: D3DFORMAT,
        rendertargetformat: D3DFORMAT,
        depthstencilformat: D3DFORMAT,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.CheckDepthStencilMatch(
                adapter,
                devicetype,
                adapterformat,
                rendertargetformat,
                depthstencilformat,
            )
        }
    }

    fn CheckDeviceFormatConversion(
        &self,
        adapter: u32,
        devicetype: D3DDEVTYPE,
        sourceformat: D3DFORMAT,
        targetformat: D3DFORMAT,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.CheckDeviceFormatConversion(
                adapter,
                devicetype,
                sourceformat,
                targetformat,
            )
        }
    }

    fn GetDeviceCaps(
        &self,
        adapter: u32,
        devicetype: D3DDEVTYPE,
        pcaps: *mut D3DCAPS9,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetDeviceCaps(adapter, devicetype, pcaps) }
    }

    fn GetAdapterMonitor(&self, adapter: u32) -> HMONITOR {
        unsafe { self.instance.GetAdapterMonitor(adapter) }
    }

    fn CreateDevice(
        &self,
        adapter: u32,
        devicetype: D3DDEVTYPE,
        hfocuswindow: HWND,
        behaviorflags: u32,
        ppresentationparameters: *mut D3DPRESENT_PARAMETERS,
        ppreturneddeviceinterface: OutRef<'_, IDirect3DDevice9>,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance.CreateDevice(
                adapter,
                devicetype,
                hfocuswindow,
                behaviorflags,
                ppresentationparameters,
                &mut ptr,
            )
        };
        let _ =
            ppreturneddeviceinterface.write(ptr.map(|x| {
                IDirect3DDevice9::from(CustomDirect3DDevice9::new(x, self.delegate.clone()))
            }));
        result
    }
}
