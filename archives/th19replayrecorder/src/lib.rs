use std::{
    fs::{create_dir_all, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

use junowen_lib::{
    hook_utils::WELL_KNOWN_VERSION_HASHES,
    structs::{app::ScreenId, input_devices::InputDevices},
    FnOfHookAssembly, Th19,
};
use th19replayplayer_lib::{FileInputList, ReplayFile};
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

static mut MODULE: HMODULE = HMODULE(0);
static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    th19: Th19,
    original_fn_from_0aba30_00fb: Option<FnOfHookAssembly>,
    replay_dir_path: String,
}

impl Props {
    fn new(th19: Th19, original_fn_from_0aba30_00fb: Option<FnOfHookAssembly>) -> Self {
        let dll_path = {
            let mut buf = [0u16; MAX_PATH as usize];
            if unsafe { GetModuleFileNameW(MODULE, &mut buf) } == 0 {
                panic!();
            }
            unsafe { PCWSTR::from_raw(buf.as_ptr()).to_string() }.unwrap()
        };

        Self {
            th19,
            original_fn_from_0aba30_00fb,
            replay_dir_path: Path::new(&dll_path)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("replays")
                .to_string_lossy()
                .to_string(),
        }
    }
}

#[derive(Default)]
struct State {
    in_game: bool,
    replay_file: ReplayFile,
    replay_file_path: PathBuf,
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
    let valid_hash = &WELL_KNOWN_VERSION_HASHES.v110c_steam;
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

fn start_recording(props: &Props, state: &mut State) {
    state.replay_file = ReplayFile::read_header_from_memory(&props.th19).unwrap();
    state.replay_file_path = Path::new(&props.replay_dir_path)
        .join(chrono::Local::now().format("%Y%m%dT%H%M%S").to_string())
        .with_extension("rep");
}

fn put<T>(vec: &mut Vec<T>, idx: usize, item: T) {
    if (idx) < vec.len() {
        vec[idx] = item;
    } else {
        vec.push(item);
    }
}

fn tick_recording(inputs: &mut FileInputList, frame: u32, input_devices: &InputDevices) {
    match inputs {
        FileInputList::HumanVsHuman(vec) => {
            let item = (
                input_devices.p1_input().current().bits() as u16,
                input_devices.p2_input().current().bits() as u16,
            );
            put(vec, frame as usize, item);
        }
        FileInputList::HumanVsCpu(vec) => {
            let item = input_devices.p1_input().current().bits() as u16;
            put(vec, frame as usize, item);
        }
    };
}

fn end_recording(props: &Props, state: &State) {
    create_dir_all(&props.replay_dir_path).unwrap();
    let mut file = BufWriter::new(File::create(&state.replay_file_path).unwrap());
    state.replay_file.write_to(&mut file).unwrap();
}

fn on_input() {
    let props = props();
    let input_devices = props.th19.input_devices();

    let state = state_mut();
    if !state.in_game {
        let Some(main_menu) = props.th19.app().main_loop_tasks().find_main_menu() else {
            return;
        };
        if main_menu.screen_id() == ScreenId::GameLoading {
            start_recording(props, state);
            state.in_game = true;
        }
    } else {
        if let Some(round_frame) = props.th19.round_frame() {
            let frame = round_frame.frame;
            tick_recording(&mut state.replay_file.inputs, frame, input_devices);
            return;
        };
        end_recording(props, state);
        state.in_game = false;
    }
}

extern "fastcall" fn from_0aba30_00fb() {
    on_input();

    let props = props();
    if let Some(func) = props.original_fn_from_0aba30_00fb {
        func()
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn Initialize(_direct_3d: *const IDirect3D9) -> bool {
    let th19 = Th19::new_hooked_process("th19.exe").unwrap();
    let (original_fn_from_0aba30_00fb, apply) = th19.hook_on_input_players(from_0aba30_00fb);
    unsafe {
        PROPS = Some(Props::new(th19, original_fn_from_0aba30_00fb));
        STATE = Some(Default::default());
    }
    apply(unsafe { &mut PROPS.as_mut().unwrap().th19 });

    true
}
