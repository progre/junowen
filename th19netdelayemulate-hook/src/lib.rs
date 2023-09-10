use std::ffi::c_void;

use bytes::{Buf, BufMut, BytesMut};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

use junowen::Th19;

static mut TH19: Option<Th19> = None;
static mut STATE: Option<State> = None;

struct State {
    p1_buffer: BytesMut,
    p2_buffer: BytesMut,
}

impl State {
    fn new() -> Self {
        let mut p1_buffer = BytesMut::with_capacity(2 * 4);
        for _ in 0..(p1_buffer.capacity() / 4) {
            p1_buffer.put_u32(0);
        }
        let mut p2_buffer = BytesMut::with_capacity(2 * 4);
        for _ in 0..(p2_buffer.capacity() / 4) {
            p2_buffer.put_u32(0);
        }
        Self {
            p1_buffer,
            p2_buffer,
        }
    }
}

fn th19() -> &'static Th19 {
    unsafe { TH19.as_ref().unwrap() }
}

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

extern "fastcall" fn hook_0abb2b() -> u32 {
    let th19 = th19();
    let state = state_mut();

    let input = th19.input_mut();

    let p1_idx = input.p1_input_idx;
    let old_p1 = state.p1_buffer.get_u32();
    let p1 = input.input_device_array[p1_idx as usize].input;
    input.input_device_array[p1_idx as usize].input = old_p1;
    state.p1_buffer.put_u32(p1);

    let p2_idx = input.p2_input_idx;
    let old_p2 = state.p2_buffer.get_u32();
    let p2 = input.input_device_array[p2_idx as usize].input;
    input.input_device_array[p2_idx as usize].input = old_p2;
    state.p2_buffer.put_u32(p2);

    // p1 の入力を返す
    input.input_device_array[p1_idx as usize].input
}

#[no_mangle]
pub extern "stdcall" fn DllMain(
    _inst_dll: HINSTANCE,
    reason: u32,
    _reserved: *const c_void,
) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        if cfg!(debug_assertions) {
            unsafe { AllocConsole() }.unwrap();
        }
        let th19 = Th19::new_hooked_process("th19.exe").unwrap();
        th19.hook_0abb2b(hook_0abb2b as _).unwrap();
        unsafe { TH19 = Some(th19) };
        unsafe { STATE = Some(State::new()) };
    }
    true
}
