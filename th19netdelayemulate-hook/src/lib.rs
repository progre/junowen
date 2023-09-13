use std::{cmp::Ordering, ffi::OsStr, io::Read, sync::mpsc, thread::spawn};

use bytes::{Buf, BufMut, BytesMut};
use interprocess::os::windows::named_pipe::{ByteReaderPipeStream, PipeListenerOptions, PipeMode};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

use junowen::Th19;

static mut TH19: Option<Th19> = None;
static mut STATE: Option<State> = None;
static mut NEW_DELAY_RECIEVER: Option<mpsc::Receiver<i8>> = None;

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

fn th19() -> &'static Th19 {
    unsafe { TH19.as_ref().unwrap() }
}

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

fn new_delay_receiver() -> &'static mpsc::Receiver<i8> {
    unsafe { NEW_DELAY_RECIEVER.as_ref().unwrap() }
}

extern "fastcall" fn hook_0abb2b() -> u32 {
    let th19 = th19();
    let state = state_mut();

    let input = th19.input_mut();

    let new_delay_receiver = new_delay_receiver();
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

    let p1_idx = input.p1_input_idx;
    if !state.p1_buffer.is_empty() {
        let old_p1 = state.p1_buffer.get_u32();
        let p1 = input.input_device_array[p1_idx as usize].input;
        input.input_device_array[p1_idx as usize].input = old_p1;
        state.p1_buffer.put_u32(p1);

        let p2_idx = input.p2_input_idx;
        let old_p2 = state.p2_buffer.get_u32();
        let p2 = input.input_device_array[p2_idx as usize].input;
        input.input_device_array[p2_idx as usize].input = old_p2;
        state.p2_buffer.put_u32(p2);
    }

    // p1 の入力を返す
    input.input_device_array[p1_idx as usize].input
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
            unsafe { AllocConsole() }.unwrap();
        }
        let th19 = Th19::new_hooked_process("th19.exe").unwrap();
        th19.hook_0aba30_00fb(hook_0abb2b).unwrap();
        let (tx, rx) = mpsc::channel();
        init_interprecess(tx);
        unsafe {
            TH19 = Some(th19);
            STATE = Some(State::new());
            NEW_DELAY_RECIEVER = Some(rx);
        }
    }
    true
}
