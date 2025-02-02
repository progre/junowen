use windows::{
    Foundation::Numerics::Matrix4x4,
    Win32::{
        Foundation::{BOOL, HANDLE, HWND, POINT, RECT},
        Graphics::{
            Direct3D9::{
                D3DBACKBUFFER_TYPE, D3DCAPS9, D3DCLIPSTATUS9, D3DDEVICE_CREATION_PARAMETERS,
                D3DDISPLAYMODE, D3DFORMAT, D3DGAMMARAMP, D3DLIGHT9, D3DMATERIAL9,
                D3DMULTISAMPLE_TYPE, D3DPOOL, D3DPRESENT_PARAMETERS, D3DPRIMITIVETYPE,
                D3DQUERYTYPE, D3DRASTER_STATUS, D3DRECT, D3DRECTPATCH_INFO, D3DRENDERSTATETYPE,
                D3DSAMPLERSTATETYPE, D3DSTATEBLOCKTYPE, D3DTEXTUREFILTERTYPE,
                D3DTEXTURESTAGESTATETYPE, D3DTRANSFORMSTATETYPE, D3DTRIPATCH_INFO,
                D3DVERTEXELEMENT9, D3DVIEWPORT9, IDirect3D9, IDirect3DBaseTexture9,
                IDirect3DCubeTexture9, IDirect3DDevice9, IDirect3DDevice9_Impl,
                IDirect3DIndexBuffer9, IDirect3DPixelShader9, IDirect3DQuery9,
                IDirect3DStateBlock9, IDirect3DSurface9, IDirect3DSwapChain9, IDirect3DTexture9,
                IDirect3DVertexBuffer9, IDirect3DVertexDeclaration9, IDirect3DVertexShader9,
                IDirect3DVolumeTexture9,
            },
            Gdi::{PALETTEENTRY, RGNDATA},
        },
    },
    core::{OutRef, Ref, implement},
};

use super::direct_3d_device_event_listener::Direct3DDeviceEventListener;

#[implement(IDirect3DDevice9)]
pub struct CustomDirect3DDevice9<T>
where
    T: 'static + Direct3DDeviceEventListener,
{
    instance: IDirect3DDevice9,
    delegate: T,
}

impl<T> CustomDirect3DDevice9<T>
where
    T: Direct3DDeviceEventListener,
{
    pub fn new(instance: IDirect3DDevice9, delegate: T) -> Self {
        Self { instance, delegate }
    }
}

impl<T> IDirect3DDevice9_Impl for CustomDirect3DDevice9_Impl<T>
where
    T: Direct3DDeviceEventListener,
{
    fn TestCooperativeLevel(&self) -> windows::core::Result<()> {
        unsafe { self.instance.TestCooperativeLevel() }
    }

    fn GetAvailableTextureMem(&self) -> u32 {
        unsafe { self.instance.GetAvailableTextureMem() }
    }

    fn EvictManagedResources(&self) -> windows::core::Result<()> {
        unsafe { self.instance.EvictManagedResources() }
    }

    fn GetDirect3D(&self) -> windows::core::Result<IDirect3D9> {
        unsafe { self.instance.GetDirect3D() }
    }

    fn GetDeviceCaps(&self, pcaps: *mut D3DCAPS9) -> windows::core::Result<()> {
        unsafe { self.instance.GetDeviceCaps(pcaps) }
    }

    fn GetDisplayMode(
        &self,
        iswapchain: u32,
        pmode: *mut D3DDISPLAYMODE,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetDisplayMode(iswapchain, pmode) }
    }

    fn GetCreationParameters(
        &self,
        pparameters: *mut D3DDEVICE_CREATION_PARAMETERS,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetCreationParameters(pparameters) }
    }

    fn SetCursorProperties(
        &self,
        xhotspot: u32,
        yhotspot: u32,
        pcursorbitmap: Ref<'_, IDirect3DSurface9>,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetCursorProperties(xhotspot, yhotspot, pcursorbitmap.as_ref())
        }
    }

    fn SetCursorPosition(&self, x: i32, y: i32, flags: u32) {
        unsafe { self.instance.SetCursorPosition(x, y, flags) }
    }

    fn ShowCursor(&self, bshow: BOOL) -> BOOL {
        unsafe { self.instance.ShowCursor(bshow.as_bool()) }
    }

    fn CreateAdditionalSwapChain(
        &self,
        ppresentationparameters: *mut D3DPRESENT_PARAMETERS,
        pswapchain: OutRef<'_, IDirect3DSwapChain9>,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance
                .CreateAdditionalSwapChain(ppresentationparameters, &mut ptr)
        };
        let _ = pswapchain.write(ptr);
        result
    }

    fn GetSwapChain(&self, iswapchain: u32) -> windows::core::Result<IDirect3DSwapChain9> {
        unsafe { self.instance.GetSwapChain(iswapchain) }
    }

    fn GetNumberOfSwapChains(&self) -> u32 {
        unsafe { self.instance.GetNumberOfSwapChains() }
    }

    fn Reset(
        &self,
        ppresentationparameters: *mut D3DPRESENT_PARAMETERS,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.Reset(ppresentationparameters) }
    }

    fn Present(
        &self,
        psourcerect: *const RECT,
        pdestrect: *const RECT,
        hdestwindowoverride: HWND,
        pdirtyregion: *const RGNDATA,
    ) -> windows::core::Result<()> {
        self.delegate.on_before_present(&self.instance);
        unsafe {
            self.instance
                .Present(psourcerect, pdestrect, hdestwindowoverride, pdirtyregion)
        }
    }

    fn GetBackBuffer(
        &self,
        iswapchain: u32,
        ibackbuffer: u32,
        r#type: D3DBACKBUFFER_TYPE,
    ) -> windows::core::Result<IDirect3DSurface9> {
        unsafe { self.instance.GetBackBuffer(iswapchain, ibackbuffer, r#type) }
    }

    fn GetRasterStatus(
        &self,
        iswapchain: u32,
        prasterstatus: *mut D3DRASTER_STATUS,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetRasterStatus(iswapchain, prasterstatus) }
    }

    fn SetDialogBoxMode(&self, benabledialogs: BOOL) -> windows::core::Result<()> {
        unsafe { self.instance.SetDialogBoxMode(benabledialogs.as_bool()) }
    }

    fn SetGammaRamp(&self, iswapchain: u32, flags: u32, pramp: *const D3DGAMMARAMP) {
        unsafe { self.instance.SetGammaRamp(iswapchain, flags, pramp) }
    }

    fn GetGammaRamp(&self, iswapchain: u32, pramp: *mut D3DGAMMARAMP) {
        unsafe { self.instance.GetGammaRamp(iswapchain, pramp) }
    }

    fn CreateTexture(
        &self,
        width: u32,
        height: u32,
        levels: u32,
        usage: u32,
        format: D3DFORMAT,
        pool: D3DPOOL,
        pptexture: OutRef<'_, IDirect3DTexture9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance.CreateTexture(
                width,
                height,
                levels,
                usage,
                format,
                pool,
                &mut ptr,
                psharedhandle,
            )
        };
        let _ = pptexture.write(ptr);
        result
    }

    fn CreateVolumeTexture(
        &self,
        width: u32,
        height: u32,
        depth: u32,
        levels: u32,
        usage: u32,
        format: D3DFORMAT,
        pool: D3DPOOL,
        ppvolumetexture: OutRef<'_, IDirect3DVolumeTexture9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance.CreateVolumeTexture(
                width,
                height,
                depth,
                levels,
                usage,
                format,
                pool,
                &mut ptr,
                psharedhandle,
            )
        };
        let _ = ppvolumetexture.write(ptr);
        result
    }

    fn CreateCubeTexture(
        &self,
        edgelength: u32,
        levels: u32,
        usage: u32,
        format: D3DFORMAT,
        pool: D3DPOOL,
        ppcubetexture: OutRef<'_, IDirect3DCubeTexture9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance.CreateCubeTexture(
                edgelength,
                levels,
                usage,
                format,
                pool,
                &mut ptr,
                psharedhandle,
            )
        };
        let _ = ppcubetexture.write(ptr);
        result
    }

    fn CreateVertexBuffer(
        &self,
        length: u32,
        usage: u32,
        fvf: u32,
        pool: D3DPOOL,
        ppvertexbuffer: OutRef<'_, IDirect3DVertexBuffer9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance
                .CreateVertexBuffer(length, usage, fvf, pool, &mut ptr, psharedhandle)
        };
        let _ = ppvertexbuffer.write(ptr);
        result
    }

    fn CreateIndexBuffer(
        &self,
        length: u32,
        usage: u32,
        format: D3DFORMAT,
        pool: D3DPOOL,
        ppindexbuffer: OutRef<'_, IDirect3DIndexBuffer9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance
                .CreateIndexBuffer(length, usage, format, pool, &mut ptr, psharedhandle)
        };
        let _ = ppindexbuffer.write(ptr);
        result
    }

    fn CreateRenderTarget(
        &self,
        width: u32,
        height: u32,
        format: D3DFORMAT,
        multisample: D3DMULTISAMPLE_TYPE,
        multisamplequality: u32,
        lockable: BOOL,
        ppsurface: OutRef<'_, IDirect3DSurface9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance.CreateRenderTarget(
                width,
                height,
                format,
                multisample,
                multisamplequality,
                lockable.as_bool(),
                &mut ptr,
                psharedhandle,
            )
        };
        let _ = ppsurface.write(ptr);
        result
    }

    fn CreateDepthStencilSurface(
        &self,
        width: u32,
        height: u32,
        format: D3DFORMAT,
        multisample: D3DMULTISAMPLE_TYPE,
        multisamplequality: u32,
        discard: BOOL,
        ppsurface: OutRef<'_, IDirect3DSurface9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance.CreateDepthStencilSurface(
                width,
                height,
                format,
                multisample,
                multisamplequality,
                discard.as_bool(),
                &mut ptr,
                psharedhandle,
            )
        };
        let _ = ppsurface.write(ptr);
        result
    }

    fn UpdateSurface(
        &self,
        psourcesurface: Ref<'_, IDirect3DSurface9>,
        psourcerect: *const RECT,
        pdestinationsurface: Ref<'_, IDirect3DSurface9>,
        pdestpoint: *const POINT,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.UpdateSurface(
                psourcesurface.as_ref(),
                psourcerect,
                pdestinationsurface.as_ref(),
                pdestpoint,
            )
        }
    }

    fn UpdateTexture(
        &self,
        psourcetexture: Ref<'_, IDirect3DBaseTexture9>,
        pdestinationtexture: Ref<'_, IDirect3DBaseTexture9>,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .UpdateTexture(psourcetexture.as_ref(), pdestinationtexture.as_ref())
        }
    }

    fn GetRenderTargetData(
        &self,
        prendertarget: Ref<'_, IDirect3DSurface9>,
        pdestsurface: Ref<'_, IDirect3DSurface9>,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetRenderTargetData(prendertarget.as_ref(), pdestsurface.as_ref())
        }
    }

    fn GetFrontBufferData(
        &self,
        iswapchain: u32,
        pdestsurface: Ref<'_, IDirect3DSurface9>,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetFrontBufferData(iswapchain, pdestsurface.as_ref())
        }
    }

    fn StretchRect(
        &self,
        psourcesurface: Ref<'_, IDirect3DSurface9>,
        psourcerect: *const RECT,
        pdestsurface: Ref<'_, IDirect3DSurface9>,
        pdestrect: *const RECT,
        filter: D3DTEXTUREFILTERTYPE,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.StretchRect(
                psourcesurface.as_ref(),
                psourcerect,
                pdestsurface.as_ref(),
                pdestrect,
                filter,
            )
        }
    }

    fn ColorFill(
        &self,
        psurface: Ref<'_, IDirect3DSurface9>,
        prect: *const RECT,
        color: u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.ColorFill(psurface.as_ref(), prect, color) }
    }

    fn CreateOffscreenPlainSurface(
        &self,
        width: u32,
        height: u32,
        format: D3DFORMAT,
        pool: D3DPOOL,
        ppsurface: OutRef<'_, IDirect3DSurface9>,
        psharedhandle: *mut HANDLE,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance.CreateOffscreenPlainSurface(
                width,
                height,
                format,
                pool,
                &mut ptr,
                psharedhandle,
            )
        };
        let _ = ppsurface.write(ptr);
        result
    }

    fn SetRenderTarget(
        &self,
        rendertargetindex: u32,
        prendertarget: Ref<'_, IDirect3DSurface9>,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetRenderTarget(rendertargetindex, prendertarget.as_ref())
        }
    }

    fn GetRenderTarget(&self, rendertargetindex: u32) -> windows::core::Result<IDirect3DSurface9> {
        unsafe { self.instance.GetRenderTarget(rendertargetindex) }
    }

    fn SetDepthStencilSurface(
        &self,
        pnewzstencil: Ref<'_, IDirect3DSurface9>,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetDepthStencilSurface(pnewzstencil.as_ref()) }
    }

    fn GetDepthStencilSurface(&self) -> windows::core::Result<IDirect3DSurface9> {
        unsafe { self.instance.GetDepthStencilSurface() }
    }

    fn BeginScene(&self) -> windows::core::Result<()> {
        unsafe { self.instance.BeginScene() }
    }

    fn EndScene(&self) -> windows::core::Result<()> {
        unsafe { self.instance.EndScene() }
    }

    fn Clear(
        &self,
        count: u32,
        prects: *const D3DRECT,
        flags: u32,
        color: u32,
        z: f32,
        stencil: u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.Clear(count, prects, flags, color, z, stencil) }
    }

    fn SetTransform(
        &self,
        state: D3DTRANSFORMSTATETYPE,
        pmatrix: *const Matrix4x4,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetTransform(state, pmatrix) }
    }

    fn GetTransform(
        &self,
        state: D3DTRANSFORMSTATETYPE,
        pmatrix: *mut Matrix4x4,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetTransform(state, pmatrix) }
    }

    fn MultiplyTransform(
        &self,
        param0: D3DTRANSFORMSTATETYPE,
        param1: *const Matrix4x4,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.MultiplyTransform(param0, param1) }
    }

    fn SetViewport(&self, pviewport: *const D3DVIEWPORT9) -> windows::core::Result<()> {
        unsafe { self.instance.SetViewport(pviewport) }
    }

    fn GetViewport(&self, pviewport: *mut D3DVIEWPORT9) -> windows::core::Result<()> {
        unsafe { self.instance.GetViewport(pviewport) }
    }

    fn SetMaterial(&self, pmaterial: *const D3DMATERIAL9) -> windows::core::Result<()> {
        unsafe { self.instance.SetMaterial(pmaterial) }
    }

    fn GetMaterial(&self, pmaterial: *mut D3DMATERIAL9) -> windows::core::Result<()> {
        unsafe { self.instance.GetMaterial(pmaterial) }
    }

    fn SetLight(&self, index: u32, param1: *const D3DLIGHT9) -> windows::core::Result<()> {
        unsafe { self.instance.SetLight(index, param1) }
    }

    fn GetLight(&self, index: u32, param1: *mut D3DLIGHT9) -> windows::core::Result<()> {
        unsafe { self.instance.GetLight(index, param1) }
    }

    fn LightEnable(&self, index: u32, enable: BOOL) -> windows::core::Result<()> {
        unsafe { self.instance.LightEnable(index, enable.as_bool()) }
    }

    fn GetLightEnable(&self, index: u32, penable: *mut BOOL) -> windows::core::Result<()> {
        unsafe { self.instance.GetLightEnable(index, penable) }
    }

    fn SetClipPlane(&self, index: u32, pplane: *const f32) -> windows::core::Result<()> {
        unsafe { self.instance.SetClipPlane(index, pplane) }
    }

    fn GetClipPlane(&self, index: u32, pplane: *mut f32) -> windows::core::Result<()> {
        unsafe { self.instance.GetClipPlane(index, pplane) }
    }

    fn SetRenderState(&self, state: D3DRENDERSTATETYPE, value: u32) -> windows::core::Result<()> {
        unsafe { self.instance.SetRenderState(state, value) }
    }

    fn GetRenderState(
        &self,
        state: D3DRENDERSTATETYPE,
        pvalue: *mut u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetRenderState(state, pvalue) }
    }

    fn CreateStateBlock(
        &self,
        r#type: D3DSTATEBLOCKTYPE,
    ) -> windows::core::Result<IDirect3DStateBlock9> {
        unsafe { self.instance.CreateStateBlock(r#type) }
    }

    fn BeginStateBlock(&self) -> windows::core::Result<()> {
        unsafe { self.instance.BeginStateBlock() }
    }

    fn EndStateBlock(&self) -> windows::core::Result<IDirect3DStateBlock9> {
        unsafe { self.instance.EndStateBlock() }
    }

    fn SetClipStatus(&self, pclipstatus: *const D3DCLIPSTATUS9) -> windows::core::Result<()> {
        unsafe { self.instance.SetClipStatus(pclipstatus) }
    }

    fn GetClipStatus(&self, pclipstatus: *mut D3DCLIPSTATUS9) -> windows::core::Result<()> {
        unsafe { self.instance.GetClipStatus(pclipstatus) }
    }

    fn GetTexture(&self, stage: u32) -> windows::core::Result<IDirect3DBaseTexture9> {
        unsafe { self.instance.GetTexture(stage) }
    }

    fn SetTexture(
        &self,
        stage: u32,
        ptexture: Ref<'_, IDirect3DBaseTexture9>,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetTexture(stage, ptexture.as_ref()) }
    }

    fn GetTextureStageState(
        &self,
        stage: u32,
        r#type: D3DTEXTURESTAGESTATETYPE,
        pvalue: *mut u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetTextureStageState(stage, r#type, pvalue) }
    }

    fn SetTextureStageState(
        &self,
        stage: u32,
        r#type: D3DTEXTURESTAGESTATETYPE,
        value: u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetTextureStageState(stage, r#type, value) }
    }

    fn GetSamplerState(
        &self,
        sampler: u32,
        r#type: D3DSAMPLERSTATETYPE,
        pvalue: *mut u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetSamplerState(sampler, r#type, pvalue) }
    }

    fn SetSamplerState(
        &self,
        sampler: u32,
        r#type: D3DSAMPLERSTATETYPE,
        value: u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetSamplerState(sampler, r#type, value) }
    }

    fn ValidateDevice(&self, pnumpasses: *mut u32) -> windows::core::Result<()> {
        unsafe { self.instance.ValidateDevice(pnumpasses) }
    }

    fn SetPaletteEntries(
        &self,
        palettenumber: u32,
        pentries: *const PALETTEENTRY,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetPaletteEntries(palettenumber, pentries) }
    }

    fn GetPaletteEntries(
        &self,
        palettenumber: u32,
        pentries: *mut PALETTEENTRY,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetPaletteEntries(palettenumber, pentries) }
    }

    fn SetCurrentTexturePalette(&self, palettenumber: u32) -> windows::core::Result<()> {
        unsafe { self.instance.SetCurrentTexturePalette(palettenumber) }
    }

    fn GetCurrentTexturePalette(&self, palettenumber: *mut u32) -> windows::core::Result<()> {
        unsafe { self.instance.GetCurrentTexturePalette(palettenumber) }
    }

    fn SetScissorRect(&self, prect: *const RECT) -> windows::core::Result<()> {
        unsafe { self.instance.SetScissorRect(prect) }
    }

    fn GetScissorRect(&self, prect: *mut RECT) -> windows::core::Result<()> {
        unsafe { self.instance.GetScissorRect(prect) }
    }

    fn SetSoftwareVertexProcessing(&self, bsoftware: BOOL) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetSoftwareVertexProcessing(bsoftware.as_bool())
        }
    }

    fn GetSoftwareVertexProcessing(&self) -> BOOL {
        unsafe { self.instance.GetSoftwareVertexProcessing() }
    }

    fn SetNPatchMode(&self, nsegments: f32) -> windows::core::Result<()> {
        unsafe { self.instance.SetNPatchMode(nsegments) }
    }

    fn GetNPatchMode(&self) -> f32 {
        unsafe { self.instance.GetNPatchMode() }
    }

    fn DrawPrimitive(
        &self,
        primitivetype: D3DPRIMITIVETYPE,
        startvertex: u32,
        primitivecount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .DrawPrimitive(primitivetype, startvertex, primitivecount)
        }
    }

    fn DrawIndexedPrimitive(
        &self,
        param0: D3DPRIMITIVETYPE,
        basevertexindex: i32,
        minvertexindex: u32,
        numvertices: u32,
        startindex: u32,
        primcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.DrawIndexedPrimitive(
                param0,
                basevertexindex,
                minvertexindex,
                numvertices,
                startindex,
                primcount,
            )
        }
    }

    fn DrawPrimitiveUP(
        &self,
        primitivetype: D3DPRIMITIVETYPE,
        primitivecount: u32,
        pvertexstreamzerodata: *const core::ffi::c_void,
        vertexstreamzerostride: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.DrawPrimitiveUP(
                primitivetype,
                primitivecount,
                pvertexstreamzerodata,
                vertexstreamzerostride,
            )
        }
    }

    fn DrawIndexedPrimitiveUP(
        &self,
        primitivetype: D3DPRIMITIVETYPE,
        minvertexindex: u32,
        numvertices: u32,
        primitivecount: u32,
        pindexdata: *const core::ffi::c_void,
        indexdataformat: D3DFORMAT,
        pvertexstreamzerodata: *const core::ffi::c_void,
        vertexstreamzerostride: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.DrawIndexedPrimitiveUP(
                primitivetype,
                minvertexindex,
                numvertices,
                primitivecount,
                pindexdata,
                indexdataformat,
                pvertexstreamzerodata,
                vertexstreamzerostride,
            )
        }
    }

    fn ProcessVertices(
        &self,
        srcstartindex: u32,
        destindex: u32,
        vertexcount: u32,
        pdestbuffer: Ref<'_, IDirect3DVertexBuffer9>,
        pvertexdecl: Ref<'_, IDirect3DVertexDeclaration9>,
        flags: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance.ProcessVertices(
                srcstartindex,
                destindex,
                vertexcount,
                pdestbuffer.as_ref(),
                pvertexdecl.as_ref(),
                flags,
            )
        }
    }

    fn CreateVertexDeclaration(
        &self,
        pvertexelements: *const D3DVERTEXELEMENT9,
    ) -> windows::core::Result<IDirect3DVertexDeclaration9> {
        unsafe { self.instance.CreateVertexDeclaration(pvertexelements) }
    }

    fn SetVertexDeclaration(
        &self,
        pdecl: Ref<'_, IDirect3DVertexDeclaration9>,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetVertexDeclaration(pdecl.as_ref()) }
    }

    fn GetVertexDeclaration(&self) -> windows::core::Result<IDirect3DVertexDeclaration9> {
        unsafe { self.instance.GetVertexDeclaration() }
    }

    fn SetFVF(&self, fvf: u32) -> windows::core::Result<()> {
        unsafe { self.instance.SetFVF(fvf) }
    }

    fn GetFVF(&self, pfvf: *mut u32) -> windows::core::Result<()> {
        unsafe { self.instance.GetFVF(pfvf) }
    }

    fn CreateVertexShader(
        &self,
        pfunction: *const u32,
    ) -> windows::core::Result<IDirect3DVertexShader9> {
        unsafe { self.instance.CreateVertexShader(pfunction) }
    }

    fn SetVertexShader(
        &self,
        pshader: Ref<'_, IDirect3DVertexShader9>,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.SetVertexShader(pshader.as_ref()) }
    }

    fn GetVertexShader(&self) -> windows::core::Result<IDirect3DVertexShader9> {
        unsafe { self.instance.GetVertexShader() }
    }

    fn SetVertexShaderConstantF(
        &self,
        startregister: u32,
        pconstantdata: *const f32,
        vector4fcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetVertexShaderConstantF(startregister, pconstantdata, vector4fcount)
        }
    }

    fn GetVertexShaderConstantF(
        &self,
        startregister: u32,
        pconstantdata: *mut f32,
        vector4fcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetVertexShaderConstantF(startregister, pconstantdata, vector4fcount)
        }
    }

    fn SetVertexShaderConstantI(
        &self,
        startregister: u32,
        pconstantdata: *const i32,
        vector4icount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetVertexShaderConstantI(startregister, pconstantdata, vector4icount)
        }
    }

    fn GetVertexShaderConstantI(
        &self,
        startregister: u32,
        pconstantdata: *mut i32,
        vector4icount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetVertexShaderConstantI(startregister, pconstantdata, vector4icount)
        }
    }

    fn SetVertexShaderConstantB(
        &self,
        startregister: u32,
        pconstantdata: *const BOOL,
        boolcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetVertexShaderConstantB(startregister, pconstantdata, boolcount)
        }
    }

    fn GetVertexShaderConstantB(
        &self,
        startregister: u32,
        pconstantdata: *mut BOOL,
        boolcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetVertexShaderConstantB(startregister, pconstantdata, boolcount)
        }
    }

    fn SetStreamSource(
        &self,
        streamnumber: u32,
        pstreamdata: Ref<'_, IDirect3DVertexBuffer9>,
        offsetinbytes: u32,
        stride: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetStreamSource(streamnumber, pstreamdata.as_ref(), offsetinbytes, stride)
        }
    }

    fn GetStreamSource(
        &self,
        streamnumber: u32,
        ppstreamdata: OutRef<'_, IDirect3DVertexBuffer9>,
        poffsetinbytes: *mut u32,
        pstride: *mut u32,
    ) -> windows::core::Result<()> {
        let mut ptr = None;
        let result = unsafe {
            self.instance
                .GetStreamSource(streamnumber, &mut ptr, poffsetinbytes, pstride)
        };
        let _ = ppstreamdata.write(ptr);
        result
    }

    fn SetStreamSourceFreq(&self, streamnumber: u32, setting: u32) -> windows::core::Result<()> {
        unsafe { self.instance.SetStreamSourceFreq(streamnumber, setting) }
    }

    fn GetStreamSourceFreq(
        &self,
        streamnumber: u32,
        psetting: *mut u32,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.GetStreamSourceFreq(streamnumber, psetting) }
    }

    fn SetIndices(&self, pindexdata: Ref<'_, IDirect3DIndexBuffer9>) -> windows::core::Result<()> {
        unsafe { self.instance.SetIndices(pindexdata.as_ref()) }
    }

    fn GetIndices(&self) -> windows::core::Result<IDirect3DIndexBuffer9> {
        unsafe { self.instance.GetIndices() }
    }

    fn CreatePixelShader(
        &self,
        pfunction: *const u32,
    ) -> windows::core::Result<IDirect3DPixelShader9> {
        unsafe { self.instance.CreatePixelShader(pfunction) }
    }

    fn SetPixelShader(&self, pshader: Ref<'_, IDirect3DPixelShader9>) -> windows::core::Result<()> {
        unsafe { self.instance.SetPixelShader(pshader.as_ref()) }
    }

    fn GetPixelShader(&self) -> windows::core::Result<IDirect3DPixelShader9> {
        unsafe { self.instance.GetPixelShader() }
    }

    fn SetPixelShaderConstantF(
        &self,
        startregister: u32,
        pconstantdata: *const f32,
        vector4fcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetPixelShaderConstantF(startregister, pconstantdata, vector4fcount)
        }
    }

    fn GetPixelShaderConstantF(
        &self,
        startregister: u32,
        pconstantdata: *mut f32,
        vector4fcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetPixelShaderConstantF(startregister, pconstantdata, vector4fcount)
        }
    }

    fn SetPixelShaderConstantI(
        &self,
        startregister: u32,
        pconstantdata: *const i32,
        vector4icount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetPixelShaderConstantI(startregister, pconstantdata, vector4icount)
        }
    }

    fn GetPixelShaderConstantI(
        &self,
        startregister: u32,
        pconstantdata: *mut i32,
        vector4icount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetPixelShaderConstantI(startregister, pconstantdata, vector4icount)
        }
    }

    fn SetPixelShaderConstantB(
        &self,
        startregister: u32,
        pconstantdata: *const BOOL,
        boolcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .SetPixelShaderConstantB(startregister, pconstantdata, boolcount)
        }
    }

    fn GetPixelShaderConstantB(
        &self,
        startregister: u32,
        pconstantdata: *mut BOOL,
        boolcount: u32,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .GetPixelShaderConstantB(startregister, pconstantdata, boolcount)
        }
    }

    fn DrawRectPatch(
        &self,
        handle: u32,
        pnumsegs: *const f32,
        prectpatchinfo: *const D3DRECTPATCH_INFO,
    ) -> windows::core::Result<()> {
        unsafe {
            self.instance
                .DrawRectPatch(handle, pnumsegs, prectpatchinfo)
        }
    }

    fn DrawTriPatch(
        &self,
        handle: u32,
        pnumsegs: *const f32,
        ptripatchinfo: *const D3DTRIPATCH_INFO,
    ) -> windows::core::Result<()> {
        unsafe { self.instance.DrawTriPatch(handle, pnumsegs, ptripatchinfo) }
    }

    fn DeletePatch(&self, handle: u32) -> windows::core::Result<()> {
        unsafe { self.instance.DeletePatch(handle) }
    }

    fn CreateQuery(&self, r#type: D3DQUERYTYPE) -> windows::core::Result<IDirect3DQuery9> {
        unsafe { self.instance.CreateQuery(r#type) }
    }
}
