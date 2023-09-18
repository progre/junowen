use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, ErrorKind, Read},
    sync::mpsc,
    thread::spawn,
};

use anyhow::Result;
use bytes::{Buf, BytesMut};
use interprocess::os::windows::named_pipe::{ByteReaderPipeStream, PipeListenerOptions, PipeMode};
use junowen_lib::{
    Battle, BattleSettings, DevicesInput, Difficulty, FnFrom0aba30_00fb, GameMode, Input, Menu,
    PlayerMatchup, ScreenId, Th19,
};
use th19replayplayer::{FileInputList, ReplayFile};
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

static mut MODULE: HMODULE = HMODULE(0);
static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    original_fn_from_0aba30_00fb: Option<FnFrom0aba30_00fb>,
    rx: mpsc::Receiver<ReplayFile>,
}

enum ReplayPlayerState<'a> {
    Standby,
    Prepare {
        th19: &'a mut Th19,
        replay_file: &'a ReplayFile,
    },
    InGame {
        th19: &'a mut Th19,
        replay_file: &'a ReplayFile,
    },
}

struct State {
    th19: Th19,
    in_game: bool,
    replay_file: Option<ReplayFile>,
}

impl State {
    fn replay_player_state(&mut self) -> ReplayPlayerState {
        match self {
            State {
                th19: _,
                in_game: false,
                replay_file: None,
            } => ReplayPlayerState::Standby,
            State {
                th19,
                in_game: false,
                replay_file: Some(replay_file),
            } => ReplayPlayerState::Prepare { th19, replay_file },
            State {
                th19,
                in_game: true,
                replay_file: Some(replay_file),
            } => ReplayPlayerState::InGame { th19, replay_file },
            State {
                th19: _,
                in_game: true,
                replay_file: None,
            } => unreachable!(),
        }
    }

    fn change_to_prepare(&mut self, replay_file: ReplayFile) {
        debug_assert!(!self.in_game);
        debug_assert!(self.replay_file.is_none());
        self.in_game = false;
        self.replay_file = Some(replay_file);
    }

    fn change_to_in_game(&mut self) {
        debug_assert!(self.replay_file.is_some());
        self.in_game = true;
    }

    fn change_to_standby(&mut self) {
        debug_assert!(self.in_game);
        debug_assert!(self.replay_file.is_some());
        self.in_game = false;
        self.replay_file = None;
    }
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

fn read_file(reader: &mut impl Read) -> Result<ReplayFile> {
    let mut buf = BytesMut::new();
    buf.clear();
    buf.resize(4, 0);
    reader.read_exact(&mut buf)?;
    let size = buf.get_u32_le();

    buf.clear();
    buf.resize(size as usize, 0);
    reader.read_exact(&mut buf)?;
    let file_path = String::from_utf8(buf.to_vec())?;

    let file = File::open(file_path)?;

    ReplayFile::read_from_reader(&mut BufReader::new(file))
}

fn init_interprecess(tx: mpsc::Sender<ReplayFile>) {
    let pipe = PipeListenerOptions::new()
        .name(OsStr::new("th19replayplayer"))
        .mode(PipeMode::Bytes)
        .create()
        .unwrap();

    spawn(move || loop {
        let mut reader: ByteReaderPipeStream = pipe.accept().unwrap();
        loop {
            let replay_file = match read_file(&mut reader) {
                Ok(ok) => ok,
                Err(err) => match err.downcast::<std::io::Error>() {
                    Ok(err) => {
                        if err.kind() == ErrorKind::BrokenPipe {
                            break;
                        }
                        eprintln!("{:?}", err);
                        break;
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                        break;
                    }
                },
            };
            tx.send(replay_file).unwrap();
        }
    });
}

fn init_battle(th19: &mut Th19, replay_file: &ReplayFile) {
    th19.set_rand_seed1(replay_file.rand_seed1).unwrap();
    th19.set_rand_seed2(replay_file.rand_seed2).unwrap();
}

fn tick_battle(
    devices_input: &mut DevicesInput,
    battle: &Battle,
    replay_file: &ReplayFile,
) -> bool {
    match &replay_file.inputs {
        FileInputList::HumanVsHuman(vec) => {
            if battle.frame as usize >= vec.len() {
                return false;
            }
            let (p1_input, p2_input) = vec[battle.frame as usize];
            devices_input.set_p1_input(Input(p1_input as u32));
            devices_input.set_p2_input(Input(p2_input as u32));
        }
        FileInputList::HumanVsCpu(vec) => {
            if battle.frame as usize >= vec.len() {
                return false;
            }
            let p1_input = vec[battle.frame as usize];
            devices_input.set_p1_input(Input(p1_input as u32));
        }
    }
    true
}

fn shot_repeatedly(prev: Input) -> Input {
    if prev.0 == Input::SHOT {
        Input(Input::NULL)
    } else {
        Input(Input::SHOT)
    }
}

fn move_cursor(input: &mut DevicesInput, current: &mut u32, target: u32) {
    if *current != target {
        *current = target;
    }
    input.set_p1_input(shot_repeatedly(input.p1_prev_input()));
    input.set_p2_input(Input(Input::NULL));
}

struct InitialBattleInformation<'a> {
    difficulty: Difficulty,
    player_matchup: PlayerMatchup,
    battle_settings: &'a BattleSettings,
    p1_character: u8,
    p1_card: u8,
    p2_character: u8,
    p2_card: u8,
}

fn move_to_battle(th19: &mut Th19, menu: &mut Menu, inits: InitialBattleInformation) -> bool {
    match (
        menu.screen_id,
        th19.game_mode().unwrap(),
        th19.player_matchup().unwrap(),
    ) {
        (ScreenId::Loading, _, _) => {
            let input = th19.input_mut();
            input.set_p1_input(Input(Input::NULL));
            input.set_p2_input(Input(Input::NULL));
            false
        }
        (ScreenId::Title, _, _) => {
            move_cursor(th19.input_mut(), &mut menu.cursor, 1);
            false
        }
        (ScreenId::PlayerMatchupSelect, _, _) => {
            let target = if inits.player_matchup == PlayerMatchup::HumanVsCpu {
                1
            } else {
                0
            };
            move_cursor(th19.input_mut(), &mut menu.cursor, target);
            false
        }
        (
            ScreenId::DifficultySelect,
            GameMode::Versus,
            PlayerMatchup::HumanVsHuman | PlayerMatchup::HumanVsCpu | PlayerMatchup::CpuVsCpu,
        ) => {
            move_cursor(th19.input_mut(), &mut menu.cursor, inits.difficulty as u32);
            false
        }
        (ScreenId::CharacterSelect, GameMode::Versus, _) => {
            let input = th19.input_mut();
            menu.p1_cursor.cursor = inits.p1_character as u32;
            th19.battle_p1_mut().set_card(inits.p1_card as u32);
            menu.p2_cursor.cursor = inits.p2_character as u32;
            th19.battle_p2_mut().set_card(inits.p2_card as u32);
            th19.put_battle_settings_in_game(inits.battle_settings)
                .unwrap();
            input.set_p1_input(shot_repeatedly(input.p1_prev_input()));
            input.set_p2_input(shot_repeatedly(input.p2_prev_input()));
            false
        }
        (ScreenId::BattleLoading, GameMode::Versus, _) => true,
        _ => {
            eprintln!("unknown screen {}", menu.screen_id as u32);
            false
        }
    }
}

fn on_input() {
    let state = state_mut();
    match state.replay_player_state() {
        ReplayPlayerState::Standby => {
            let Ok(ok) = props().rx.try_recv() else {
                return;
            };
            state.change_to_prepare(ok);
            on_input();
        }
        ReplayPlayerState::Prepare { th19, replay_file } => {
            let Some(menu) = th19.game().game_mains.find_menu_mut() else {
                return;
            };
            if move_to_battle(
                th19,
                menu,
                InitialBattleInformation {
                    difficulty: replay_file.difficulty,
                    player_matchup: replay_file.player_matchup,
                    battle_settings: &replay_file.battle_settings,
                    p1_character: replay_file.p1_character,
                    p1_card: replay_file.p1_card,
                    p2_character: replay_file.p2_character,
                    p2_card: replay_file.p2_card,
                },
            ) {
                init_battle(th19, replay_file);
                state.change_to_in_game();
            }
        }
        ReplayPlayerState::InGame { th19, replay_file } => {
            if let Some(battle) = th19.battle() {
                if tick_battle(th19.input_mut(), battle, replay_file) {
                    return;
                }
            }
            state.change_to_standby();
        }
    }
}

extern "fastcall" fn from_0aba30_00fb() -> u32 {
    on_input();

    let props = props();
    if let Some(func) = props.original_fn_from_0aba30_00fb {
        func()
    } else {
        state().th19.input().p1_input().0 // p1 の入力を返す
    }
}

#[no_mangle]
pub extern "stdcall" fn DllMain(inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        if cfg!(debug_assertions) {
            let _ = unsafe { AllocConsole() };
            std::env::set_var("RUST_BACKTRACE", "1");
        }
        unsafe { MODULE = inst_dll.into() };
        let (tx, rx) = mpsc::channel();
        init_interprecess(tx);

        let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();
        let original_fn_from_0aba30_00fb = th19.hook_0aba30_00fb(from_0aba30_00fb).unwrap();
        unsafe {
            PROPS = Some(Props {
                original_fn_from_0aba30_00fb,
                rx,
            });
            STATE = Some(State {
                th19,
                in_game: false,
                replay_file: None,
            });
        }
    }
    true
}
