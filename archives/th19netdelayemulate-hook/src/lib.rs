use std::{cmp::Ordering, ffi::OsStr, io::Read, mem::transmute, sync::mpsc, thread::spawn};

use bytes::{Buf, BufMut, BytesMut};
use interprocess::os::windows::named_pipe::{ByteReaderPipeStream, PipeListenerOptions, PipeMode};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

use junowen_lib::{Input, Th19};

static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    th19: Th19,
    original_fn_from_0aba30_00fb: Option<usize>,
    new_delay_receiver: mpsc::Receiver<i8>,
}

struct State {
    p1_buffer: BytesMut,
    p2_buffer: BytesMut,
}

impl State {
    fn new() -> Self {
        Self {
            p1_buffer: BytesMut::new(),
            p2_buffer: BytesMut::new(),
        }
    }
}

fn props() -> &'static Props {
    unsafe { PROPS.as_ref().unwrap() }
}

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

extern "fastcall" fn hook_0abb2b() -> u32 {
    let th19 = &props().th19;
    let state = state_mut();

    let input = th19.input_mut();

    let new_delay_receiver = &props().new_delay_receiver;
    if let Ok(delay) = new_delay_receiver.try_recv() {
        let old_delay = state.p1_buffer.len() / 4;
        println!("old delay: {}, new delay: {}", old_delay, delay);
        let delay = delay as usize;
        match delay.cmp(&old_delay) {
            Ordering::Less => {
                let skip = (old_delay - delay) * 4;
                state.p1_buffer.advance(skip);
                state.p2_buffer.advance(skip);
            }
            Ordering::Greater => {
                for _ in 0..(delay - old_delay) {
                    state.p1_buffer.put_u32(0);
                    state.p2_buffer.put_u32(0);
                }
            }
            Ordering::Equal => (),
        }
    }

    if !state.p1_buffer.is_empty() {
        let old_p1 = Input(state.p1_buffer.get_u32());
        let p1 = input.p1_input();
        input.set_p1_input(old_p1);
        state.p1_buffer.put_u32(p1.0);

        let old_p2 = Input(state.p2_buffer.get_u32());
        let p2 = input.p2_input();
        input.set_p2_input(old_p2);
        state.p2_buffer.put_u32(p2.0);
    }

    if let Some(func) = props().original_fn_from_0aba30_00fb {
        type Func = fn() -> u32;
        let func: Func = unsafe { transmute(func) };
        func()
    } else {
        input.p1_input().0 // p1 の入力を返す
    }
}

fn init_interprecess(tx: mpsc::Sender<i8>) {
    let pipe = PipeListenerOptions::new()
        .name(OsStr::new("th19netdelayemulate"))
        .mode(PipeMode::Bytes)
        .create()
        .unwrap();

    let mut buf = [0; 1];
    spawn(move || loop {
        let mut reader: ByteReaderPipeStream = pipe.accept().unwrap();
        reader.read_exact(&mut buf).unwrap();
        println!("pipe received {}", buf[0]);
        tx.send(buf[0] as i8).unwrap();
    });
}

#[no_mangle]
pub extern "stdcall" fn DllMain(_inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        if cfg!(debug_assertions) {
            let _ = unsafe { AllocConsole() };
        }
        let th19 = Th19::new_hooked_process("th19.exe").unwrap();
        let original_fn_from_0aba30_00fb = th19.hook_0aba30_00fb(hook_0abb2b).unwrap();
        let (tx, rx) = mpsc::channel();
        init_interprecess(tx);
        unsafe {
            PROPS = Some(Props {
                th19,
                original_fn_from_0aba30_00fb,
                new_delay_receiver: rx,
            });
            STATE = Some(State::new());
        }
    }
    true
}