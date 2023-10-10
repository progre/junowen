use std::{f32::consts::PI, ffi::c_void, mem::size_of};

use junowen_lib::{Fn0a9000, Th19};
use windows::Win32::{
    Foundation::HINSTANCE,
    Graphics::Direct3D9::{D3DFVF_DIFFUSE, D3DFVF_XYZRHW, D3DPT_TRIANGLEFAN},
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

static mut TH19: Option<Th19> = None;
static mut STATE: Option<State> = None;

struct State {
    original_fn_0a9000: Fn0a9000,
    buttons: Vec<Vec<SimpleVertex>>,
}

impl State {
    fn new(original_fn_0a9000: Fn0a9000) -> Self {
        let mut buttons = Vec::new();
        let mut i = 0;
        let mut color = 0xffff2800;
        for _ in 0..2 {
            for _ in 0..4 {
                buttons.push(create_vertex(i as f32 * 16.0 + 8.0, color, CIRCLE));
                i += 1;
            }
            buttons.push(create_vertex(i as f32 * 16.0 + 8.0, color, UP));
            i += 1;
            buttons.push(create_vertex(i as f32 * 16.0 + 8.0, color, DOWN));
            i += 1;
            buttons.push(create_vertex(i as f32 * 16.0 + 8.0, color, LEFT));
            i += 1;
            buttons.push(create_vertex(i as f32 * 16.0 + 8.0, color, RIGHT));
            i += 1;
            buttons.push(create_vertex(i as f32 * 16.0 + 8.0, color, CIRCLE));
            i += 2;
            color = 0xff66ccff;
        }

        Self {
            original_fn_0a9000,
            buttons,
        }
    }
}

struct SimpleVertex {
    _x: f32,
    _y: f32,
    _z: f32,
    _rhw: f32,
    _color: u32,
}

fn th19() -> &'static Th19 {
    unsafe { TH19.as_ref().unwrap() }
}

fn state() -> &'static State {
    unsafe { STATE.as_ref().unwrap() }
}

fn sin_cos(theta: f32, invert: bool) -> (f32, f32) {
    (theta.sin(), theta.cos() * if invert { -1.0 } else { 1.0 })
}

fn cos_sin(theta: f32, invert: bool) -> (f32, f32) {
    (theta.cos() * if invert { -1.0 } else { 1.0 }, theta.sin())
}

type Shape = (i32, fn(f32, bool) -> (f32, f32), bool);

const CIRCLE: Shape = (16, sin_cos, false);
const UP: Shape = (3, sin_cos, true);
const DOWN: Shape = (3, sin_cos, false);
const LEFT: Shape = (3, cos_sin, true);
const RIGHT: Shape = (3, cos_sin, false);

fn create_vertex(center_x: f32, color: u32, (sides, sin_cos, invert): Shape) -> Vec<SimpleVertex> {
    let r = 8.0;
    let center_y = r;

    [SimpleVertex {
        _x: center_x,
        _y: center_y,
        _z: 0.0,
        _rhw: 1.0,
        _color: color,
    }]
    .into_iter()
    .chain((0..sides).chain([0]).map(|i| {
        let theta = 2.0 * PI * (i as f32) / (sides as f32);
        let (x, y) = sin_cos(theta, invert);
        let x = center_x + x * r;
        let y = center_y + y * r;
        SimpleVertex {
            _x: x,
            _y: y,
            _z: 0.0,
            _rhw: 1.0,
            _color: color,
        }
    }))
    .collect()
}

extern "thiscall" fn hook_0a9000(this: *const c_void) {
    let state = state();

    (state.original_fn_0a9000)(this);

    let th19 = th19();
    let p1 = th19.p1_input();
    let p2 = th19.p2_input();

    let device = th19.direct_3d_device().unwrap();

    if unsafe { device.SetFVF(D3DFVF_XYZRHW | D3DFVF_DIFFUSE) }.is_err() {
        eprintln!("SetFVF failed");
    }
    for (i, button) in state.buttons.iter().enumerate() {
        if i < 9 {
            if ((p1.0 >> i) & 0x01) == 0 {
                continue;
            }
        } else {
            //
            if ((p2.0 >> (i - 9)) & 0x01) == 0 {
                continue;
            }
        }
        if unsafe {
            device.DrawPrimitiveUP(
                D3DPT_TRIANGLEFAN,
                button.len() as u32 - 2,
                button.as_ptr() as *const c_void,
                size_of::<SimpleVertex>() as u32,
            )
        }
        .is_err()
        {
            eprintln!("DrawPrimitiveUP failed");
        }
    }
}

#[no_mangle]
pub extern "stdcall" fn DllMain(_inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        if cfg!(debug_assertions) {
            let _ = unsafe { AllocConsole() };
        }
        let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();
        let (original_fn_0a9000, apply) = th19.hook_0a9540_0175(hook_0a9000);
        unsafe { TH19 = Some(th19) };
        unsafe { STATE = Some(State::new(original_fn_0a9000)) };
        apply(unsafe { TH19.as_mut().unwrap() });
    }
    true
}
