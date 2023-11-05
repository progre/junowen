mod character_selecter;
mod file;
mod settings_editor;

use std::path::Path;

use junowen_lib::{
    hook_utils::WELL_KNOWN_VERSION_HASHES, Fn002530, Fn009fa0, Fn012480, GameSettings, Th19,
};
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
static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    settings_path: String,
    original_fn_from_13f9d0_0446: Fn009fa0,
    original_fn_from_107540_0046: Fn012480,
    original_fn_from_107540_0937: Fn002530,
}

impl Props {
    fn new(
        original_fn_from_13f9d0_0446: Fn009fa0,
        original_fn_from_107540_0046: Fn012480,
        original_fn_from_107540_0937: Fn002530,
    ) -> Self {
        let dll_path = {
            let mut buf = [0u16; MAX_PATH as usize];
            if unsafe { GetModuleFileNameW(MODULE, &mut buf) } == 0 {
                panic!();
            }
            unsafe { PCWSTR::from_raw(buf.as_ptr()).to_string() }.unwrap()
        };

        Self {
            settings_path: Path::new(&dll_path)
                .with_extension("cfg")
                .to_string_lossy()
                .to_string(),
            original_fn_from_13f9d0_0446,
            original_fn_from_107540_0046,
            original_fn_from_107540_0937,
        }
    }
}

struct State {
    th19: Th19,
    tmp_battle_settings: GameSettings,
}

fn props() -> &'static Props {
    unsafe { PROPS.as_ref().unwrap() }
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
    let valid_hash = &WELL_KNOWN_VERSION_HASHES.v100a_steam;
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
    let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();
    let (original_fn_from_13f9d0_0446, apply_hook_13f9d0_0446) =
        th19.hook_13f9d0_0446(post_read_battle_settings_from_menu_to_game);
    let (original_fn_from_107540_0046, apply_hook_107540_0046) =
        th19.hook_107540_0046(on_open_settings_editor);
    let (original_fn_from_107540_0937, apply_hook_107540_0937) =
        th19.hook_107540_0937(on_close_settings_editor);
    unsafe {
        PROPS = Some(Props::new(
            original_fn_from_13f9d0_0446,
            original_fn_from_107540_0046,
            original_fn_from_107540_0937,
        ));
        STATE = Some(State {
            th19,
            tmp_battle_settings: Default::default(),
        });
    }
    let th19 = &mut state_mut().th19;
    apply_hook_13f9d0_0446(th19);
    apply_hook_107540_0046(th19);
    apply_hook_107540_0937(th19);

    true
}
