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
    th19_helpers::{select_cursor, shot_repeatedly, AutomaticInputs},
    Difficulty, FnOfHookAssembly, GameMode, GameSettings, Input, InputDevices, Menu, PlayerMatchup,
    Round, ScreenId, Th19,
};
use th19replayplayer_lib::{FileInputList, ReplayFile};
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

static mut MODULE: HMODULE = HMODULE(0);
static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    original_on_input_menu: Option<FnOfHookAssembly>,
    original_on_input_players: Option<FnOfHookAssembly>,
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

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

struct InitialBattleInformation<'a> {
    pub difficulty: Difficulty,
    pub player_matchup: PlayerMatchup,
    pub battle_settings: &'a GameSettings,
    pub p1_character: u8,
    pub p1_card: u8,
    pub p2_character: u8,
    pub p2_card: u8,
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

fn tick_battle(input_devices: &mut InputDevices, battle: &Round, replay_file: &ReplayFile) -> bool {
    match &replay_file.inputs {
        FileInputList::HumanVsHuman(vec) => {
            if battle.frame as usize >= vec.len() {
                return false;
            }
            let (p1_input, p2_input) = vec[battle.frame as usize];
            input_devices.set_p1_input(Input(p1_input as u32));
            input_devices.set_p2_input(Input(p2_input as u32));
        }
        FileInputList::HumanVsCpu(vec) => {
            if battle.frame as usize >= vec.len() {
                return false;
            }
            let p1_input = vec[battle.frame as usize];
            input_devices.set_p1_input(Input(p1_input as u32));
        }
    }
    true
}

fn move_to_battle_menu_input(
    th19: &mut Th19,
    menu: &mut Menu,
    inits: &InitialBattleInformation,
) -> bool {
    match (
        menu.screen_id,
        th19.selection().game_mode,
        th19.selection().player_matchup,
    ) {
        (ScreenId::TitleLoading, _, _)
        | (ScreenId::Title, _, _)
        | (ScreenId::PlayerMatchupSelect, _, _) => {
            AutomaticInputs::TransitionToLocalVersusDifficultySelect(inits.player_matchup)
                .on_input_menu(th19, menu);
            false
        }
        (
            ScreenId::DifficultySelect,
            GameMode::Versus,
            PlayerMatchup::HumanVsHuman | PlayerMatchup::HumanVsCpu | PlayerMatchup::CpuVsCpu,
        ) => {
            th19.set_menu_input(select_cursor(
                th19.prev_menu_input(),
                &mut menu.cursor,
                inits.difficulty as u32,
            ));
            false
        }
        (ScreenId::CharacterSelect, GameMode::Versus, _) => {
            th19.set_menu_input(Input::NULL.into());
            false
        }
        (ScreenId::GameLoading, GameMode::Versus, _) => true,
        _ => {
            eprintln!("unknown screen {}", menu.screen_id as u32);
            false
        }
    }
}

fn move_to_battle_player_inputs(
    th19: &mut Th19,
    menu: &mut Menu,
    inits: &InitialBattleInformation,
) -> bool {
    let input_devices = th19.input_devices_mut();
    match (
        menu.screen_id,
        th19.selection().game_mode,
        th19.selection().player_matchup,
    ) {
        (ScreenId::TitleLoading, _, _)
        | (ScreenId::Title, _, _)
        | (ScreenId::PlayerMatchupSelect, _, _)
        | (ScreenId::DifficultySelect, _, _) => {
            input_devices.set_p1_input(Input::NULL.into());
            input_devices.set_p2_input(Input::NULL.into());
            false
        }
        (ScreenId::CharacterSelect, GameMode::Versus, _) => {
            menu.p1_cursor.cursor = inits.p1_character as u32;
            th19.selection_mut().p1_mut().card = inits.p1_card as u32;
            menu.p2_cursor.cursor = inits.p2_character as u32;
            th19.selection_mut().p2_mut().card = inits.p2_card as u32;
            th19.put_game_settings_in_game(inits.battle_settings)
                .unwrap();
            input_devices.set_p1_input(shot_repeatedly(input_devices.p1_prev_input()));
            input_devices.set_p2_input(shot_repeatedly(input_devices.p2_prev_input()));
            false
        }
        (ScreenId::GameLoading, GameMode::Versus, _) => true,
        _ => {
            eprintln!("unknown screen {}", menu.screen_id as u32);
            false
        }
    }
}

fn on_input_players_internal() {
    let state = state_mut();
    match state.replay_player_state() {
        ReplayPlayerState::Standby => {
            let Ok(ok) = props().rx.try_recv() else {
                return;
            };
            state.change_to_prepare(ok);
            on_input_players_internal();
        }
        ReplayPlayerState::Prepare { th19, replay_file } => {
            let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
                return;
            };
            if move_to_battle_player_inputs(
                th19,
                menu,
                &InitialBattleInformation {
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
            if let Some(battle) = th19.round() {
                if tick_battle(th19.input_devices_mut(), battle, replay_file) {
                    return;
                }
            }
            state.change_to_standby();
        }
    }
}

extern "fastcall" fn on_input_players() {
    on_input_players_internal();

    let props = props();
    if let Some(func) = props.original_on_input_players {
        func()
    }
}

extern "fastcall" fn on_input_menu() {
    let state = state_mut();
    match state.replay_player_state() {
        ReplayPlayerState::Standby => {
            return;
        }
        ReplayPlayerState::Prepare { th19, replay_file } => {
            let Some(menu) = th19.app_mut().main_loop_tasks_mut().find_menu_mut() else {
                return;
            };
            move_to_battle_menu_input(
                th19,
                menu,
                &InitialBattleInformation {
                    difficulty: replay_file.difficulty,
                    player_matchup: replay_file.player_matchup,
                    battle_settings: &replay_file.battle_settings,
                    p1_character: replay_file.p1_character,
                    p1_card: replay_file.p1_card,
                    p2_character: replay_file.p2_character,
                    p2_card: replay_file.p2_card,
                },
            );
        }
        ReplayPlayerState::InGame { .. } => {}
    }

    let props = props();
    if let Some(func) = props.original_on_input_menu {
        func()
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
        let (original_on_input_menu, apply_hook_on_input_menu) =
            th19.hook_on_input_menu(on_input_menu);
        let (original_on_input_players, apply_hook_on_input_players) =
            th19.hook_on_input_players(on_input_players);
        unsafe {
            PROPS = Some(Props {
                original_on_input_menu,
                original_on_input_players,
                rx,
            });
            STATE = Some(State {
                th19,
                in_game: false,
                replay_file: None,
            });
        }
        let th19 = &mut state_mut().th19;
        apply_hook_on_input_players(th19);
        apply_hook_on_input_menu(th19);
    }
    true
}
