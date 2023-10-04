use junowen_lib::{reset_cursors, FnOfHookAssembly, Th19};
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    Graphics::Direct3D9::IDirect3D9,
    System::{
        Console::AllocConsole,
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

static mut MODULE: HMODULE = HMODULE(0);
static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    old_on_waiting_online_vs_connection: Option<FnOfHookAssembly>,
}

struct State {
    th19: Th19,
}

fn props() -> &'static Props {
    unsafe { PROPS.as_ref().unwrap() }
}

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

#[no_mangle]
pub extern "stdcall" fn DllMain(inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    match reason {
        DLL_PROCESS_ATTACH => {
            unsafe { MODULE = inst_dll.into() };
        }
        DLL_PROCESS_DETACH => {}
        _ => {}
    }
    true
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn CheckVersion(hash: *const u8, length: usize) -> bool {
    let valid_hash: [u8; 28] = [
        0xaa, 0x4e, 0xf4, 0xe6, 0xfa, 0xe1, 0x23, 0xcb, 0xcb, 0xc1, 0xc2, 0xc2, 0x32, 0x46, 0x2d,
        0x5e, 0xfa, 0x6b, 0x21, 0x5d, 0x4a, 0x94, 0xf6, 0x4d, 0x62, 0xbc, 0xef, 0xcb,
    ];
    if length != valid_hash.len() {
        return false;
    }
    for (i, &valid_hash_byte) in valid_hash.iter().enumerate() {
        if unsafe { *(hash.wrapping_add(i)) } != valid_hash_byte {
            return false;
        }
    }
    true
}

extern "fastcall" fn on_waiting_online_vs_connection() {
    reset_cursors(&mut state_mut().th19);

    if let Some(func) = props().old_on_waiting_online_vs_connection {
        func()
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn Initialize(_direct_3d: *const IDirect3D9) -> bool {
    if cfg!(debug_assertions) {
        let _ = unsafe { AllocConsole() };
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();
    let (old_on_waiting_online_vs_connection, apply) =
        th19.hook_on_waiting_online_vs_connection(on_waiting_online_vs_connection);
    unsafe {
        PROPS = Some(Props {
            old_on_waiting_online_vs_connection,
        });
        STATE = Some(State { th19 });
    }
    apply(&mut state_mut().th19);

    true
}
