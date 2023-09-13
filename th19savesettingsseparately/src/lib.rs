mod character_selecter;
mod file;
mod settings_editor;

use std::path::Path;

use junowen::{BattleSettings, Th19};
use settings_editor::{on_close_settings_editor, on_open_settings_editor};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, HMODULE, MAX_PATH},
        Graphics::Direct3D9::IDirect3D9,
        System::{
            LibraryLoader::GetModuleFileNameW,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
};

use character_selecter::post_read_battle_settings_from_menu_to_game;

static mut MODULE: HMODULE = HMODULE(0);
static mut PROP: Option<Prop> = None;
static mut STATE: Option<State> = None;

struct Prop {
    th19: Th19,
    settings_path: String,
    original_fn_from_13fe16: usize,
    original_fn_from_107540_0046: usize,
    original_fn_from_107540_0937: usize,
}

impl Prop {
    fn new(
        th19: Th19,
        original_fn_from_13fe16: usize,
        original_fn_from_107540_0046: usize,
        original_fn_from_107540_0937: usize,
    ) -> Self {
        let dll_path = {
            let mut buf = [0u16; MAX_PATH as usize];
            if unsafe { GetModuleFileNameW(MODULE, &mut buf) } == 0 {
                panic!();
            }
            unsafe { PCWSTR::from_raw(buf.as_ptr()).to_string() }.unwrap()
        };

        Self {
            th19,
            settings_path: Path::new(&dll_path)
                .with_extension("cfg")
                .to_string_lossy()
                .to_string(),
            original_fn_from_13fe16,
            original_fn_from_107540_0046,
            original_fn_from_107540_0937,
        }
    }
}

#[derive(Default)]
struct State {
    tmp_battle_settings: BattleSettings,
}

fn prop() -> &'static Prop {
    unsafe { PROP.as_ref().unwrap() }
}

fn state() -> &'static State {
    unsafe { STATE.as_ref().unwrap() }
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

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn Initialize(_direct_3d: *const IDirect3D9) -> bool {
    let th19 = Th19::new_hooked_process("th19.exe").unwrap();
    let original_fn_from_13fe16 = th19
        .hook_13f9d0_0446(post_read_battle_settings_from_menu_to_game)
        .unwrap();
    let original_fn_from_107540_0046 = th19.hook_107540_0046(on_open_settings_editor).unwrap();
    let original_fn_from_107540_0937 = th19.hook_107540_0937(on_close_settings_editor).unwrap();
    unsafe {
        PROP = Some(Prop::new(
            th19,
            original_fn_from_13fe16,
            original_fn_from_107540_0046,
            original_fn_from_107540_0937,
        ));
        STATE = Some(Default::default());
    }

    true
}
