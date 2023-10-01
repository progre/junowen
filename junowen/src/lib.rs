mod cui;
mod interprocess;
mod session;

use std::{ffi::c_void, sync::mpsc};

use junowen_lib::{
    move_to_local_versus_difficulty_select, Fn009fa0, Fn1049e0, FnOfHookAssembly, Input,
    PlayerMatchup, ScreenId, Th19,
};
use session::{MatchInitial, RandomNumberInitial, Session};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{Console::AllocConsole, SystemServices::DLL_PROCESS_ATTACH},
};

use crate::interprocess::init_interprocess;

static mut PROPS: Option<Props> = None;
static mut STATE: Option<State> = None;

struct Props {
    old_on_input_players: Option<FnOfHookAssembly>,
    old_on_input_menu: Option<FnOfHookAssembly>,
    old_fn_from_11f870_034c: Fn1049e0,
    old_fn_from_13f9d0_0446: Fn009fa0,
    session_receiver: mpsc::Receiver<Session>,
}

enum NetBattleState<'a> {
    Standby,
    Prepare,
    Select {
        session: &'a mut Session,
        th19: &'a mut Th19,
        match_initial: &'a mut Option<MatchInitial>,
        first_time: bool,
    },
    GameLoading,
    Game {
        session: &'a mut Session,
        th19: &'a mut Th19,
    },
    BackToSelect,
}

struct State {
    th19: Th19,
    state: u8,
    session: Option<Session>,
    match_initial: Option<MatchInitial>,
}

impl State {
    pub fn net_battle_state(&mut self) -> NetBattleState {
        match self.state {
            0x00 => NetBattleState::Standby,
            0x10 => NetBattleState::Prepare,
            0x20 | 0x21 => NetBattleState::Select {
                session: self.session.as_mut().unwrap(),
                th19: &mut self.th19,
                match_initial: &mut self.match_initial,
                first_time: self.state == 0x20,
            },
            0x30 => NetBattleState::GameLoading,
            0x40 => NetBattleState::Game {
                session: self.session.as_mut().unwrap(),
                th19: &mut self.th19,
            },
            0x50 => NetBattleState::BackToSelect,
            _ => unreachable!(),
        }
    }

    pub fn start_session(&mut self, session: Session) {
        self.state = 0x10;
        self.session = Some(session);
    }

    pub fn change_to_select(&mut self) {
        self.state = 0x20;
    }

    pub fn change_to_select_after_first_time(&mut self) {
        self.state = 0x21;
    }

    pub fn change_to_game_loading(&mut self) {
        self.state = 0x30;
    }

    pub fn change_to_game(&mut self) {
        self.state = 0x40;
    }

    pub fn change_to_back_to_select(&mut self) {
        self.state = 0x50;
    }

    pub fn end_session(&mut self) {
        self.state = 0x00;
        self.session = None;
        self.match_initial = None;
    }
}

fn props() -> &'static Props {
    unsafe { PROPS.as_ref().unwrap() }
}

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

fn on_input_players_impl() {
    let state = state_mut();
    let menu = 'l: {
        match state.net_battle_state() {
            NetBattleState::Standby => {
                let Ok(session) = props().session_receiver.try_recv() else {
                    return;
                };
                state.start_session(session);
                None
            }
            NetBattleState::Prepare => {
                let Some(menu) = state.th19.app().main_loop_tasks.find_menu_mut() else {
                    break 'l None;
                };
                if menu.screen_id == ScreenId::DifficultySelect {
                    state.change_to_select();
                }
                Some(menu)
            }
            NetBattleState::Select { first_time, .. } => {
                if first_time {
                    state.change_to_select_after_first_time();
                }
                let menu = state.th19.app().main_loop_tasks.find_menu_mut().unwrap();
                if menu.screen_id == ScreenId::GameLoading {
                    state.change_to_game_loading();
                }
                if menu.screen_id == ScreenId::PlayerMatchupSelect {
                    state.end_session();
                    return;
                }
                Some(menu)
            }
            NetBattleState::GameLoading => {
                let Some(game) = state.th19.game() else {
                    break 'l None;
                };
                if !game.is_first_frame() {
                    break 'l None;
                }
                state.change_to_game();
                None
            }
            NetBattleState::Game { session: _, th19 } => {
                if th19.game().is_some() {
                    break 'l None;
                }
                println!("change_to_back_to_select");
                state.change_to_back_to_select();
                None
            }
            NetBattleState::BackToSelect => {
                let Some(menu) = state.th19.app().main_loop_tasks.find_menu_mut() else {
                    break 'l None;
                };
                if menu.screen_id == ScreenId::CharacterSelect {
                    state.change_to_select();
                }
                Some(menu)
            }
        }
    };
    match state.net_battle_state() {
        NetBattleState::Standby => unreachable!(),
        NetBattleState::Prepare => {}
        NetBattleState::Select {
            first_time,
            session,
            th19,
            match_initial,
        } => {
            if first_time {
                th19.set_difficulty_cursor(1).unwrap();
                th19.p1_mut().character = 0;
                th19.p2_mut().character = 1;
                for player_select in th19.app().main_loop_tasks.player_selects_mut() {
                    player_select.player.card = 0;
                }

                if session.host() {
                    if match_initial.is_none() {
                        let init = MatchInitial {
                            game_settings: th19.game_settings_in_menu().unwrap(),
                        };
                        session.send_init_match(init.clone());
                        *match_initial = Some(init);
                    }
                    session.send_init_random_number(RandomNumberInitial {
                        seed1: th19.rand_seed1().unwrap(),
                        seed2: th19.rand_seed2().unwrap(),
                        seed3: th19.rand_seed3().unwrap(),
                        seed4: th19.rand_seed4().unwrap(),
                        seed5: th19.rand_seed5().unwrap(),
                        seed6: th19.rand_seed6().unwrap(),
                        seed7: th19.rand_seed7().unwrap(),
                        seed8: th19.rand_seed8().unwrap(),
                    });
                } else {
                    if match_initial.is_none() {
                        *match_initial = Some(session.recv_init_match().unwrap());
                    }
                    let (init, delay_remainings) = session.recv_init_round().unwrap();
                    println!("delay_remainings: {}", delay_remainings);
                    th19.set_rand_seed1(init.seed1).unwrap();
                    th19.set_rand_seed2(init.seed2).unwrap();
                    th19.set_rand_seed3(init.seed3).unwrap();
                    th19.set_rand_seed4(init.seed4).unwrap();
                    th19.set_rand_seed5(init.seed5).unwrap();
                    th19.set_rand_seed6(init.seed6).unwrap();
                    th19.set_rand_seed7(init.seed7).unwrap();
                    th19.set_rand_seed8(init.seed8).unwrap();
                }
            }

            let menu = menu.unwrap();
            if menu.screen_id == ScreenId::DifficultySelect {
                return;
            }

            session.enqueue_input(th19.input().p1_input().0 as u8);
            let (p1, p2) = match session.dequeue_inputs() {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("session aborted: {}", err);
                    state.end_session();
                    return;
                }
            };
            let input = th19.input_mut();
            input.set_p1_input(Input(p1 as u32));
            input.set_p2_input(Input(p2 as u32));
        }
        NetBattleState::GameLoading => {}
        NetBattleState::Game { session, th19 } => {
            // -1フレーム目、0フレーム目は複数回呼ばれ、回数が不定なのでスキップする
            if th19.game().unwrap().frame >= 1 {
                session.enqueue_input(th19.input().p1_input().0 as u8);
                let (p1, p2) = match session.dequeue_inputs() {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("session aborted: {}", err);
                        state.end_session();
                        return;
                    }
                };
                let input = th19.input_mut();
                input.set_p1_input(Input(p1 as u32));
                input.set_p2_input(Input(p2 as u32));
            }
        }
        NetBattleState::BackToSelect => {}
    }
}

fn on_input_menu_impl() {
    let state = state_mut();
    match state.net_battle_state() {
        NetBattleState::Standby => {}
        NetBattleState::Prepare => {
            let Some(menu) = state.th19.app().main_loop_tasks.find_menu_mut() else {
                return;
            };

            move_to_local_versus_difficulty_select(
                &mut state.th19,
                menu,
                PlayerMatchup::HumanVsHuman,
            );
        }
        NetBattleState::Select {
            first_time: _,
            session,
            th19,
            match_initial: _,
        } => {
            let menu = th19.app().main_loop_tasks.find_menu_mut().unwrap();
            if menu.screen_id != ScreenId::DifficultySelect {
                return;
            }

            session.enqueue_input(if session.host() {
                th19.menu_input().0 as u8
            } else {
                Input::NULL as u8
            });
            let (p1, _p2) = match session.dequeue_inputs() {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("session aborted: {}", err);
                    state.end_session();
                    return;
                }
            };
            *th19.menu_input_mut() = Input(p1 as u32);
        }
        NetBattleState::GameLoading => {}
        NetBattleState::Game { .. } => {}
        NetBattleState::BackToSelect => {}
    }
}
extern "fastcall" fn on_input_players() {
    on_input_players_impl();

    let props = props();
    if let Some(func) = props.old_on_input_players {
        func()
    }
}

extern "fastcall" fn on_input_menu() {
    on_input_menu_impl();

    let props = props();
    if let Some(func) = props.old_on_input_menu {
        func()
    }
}

extern "fastcall" fn on_round_over() {
    (props().old_fn_from_11f870_034c)();

    let state = state_mut();
    if let Some(session) = &mut state.session {
        let th19 = &mut state.th19;
        if session.host() {
            session.send_init_random_number(RandomNumberInitial {
                seed1: th19.rand_seed1().unwrap(),
                seed2: th19.rand_seed2().unwrap(),
                seed3: th19.rand_seed3().unwrap(),
                seed4: th19.rand_seed4().unwrap(),
                seed5: th19.rand_seed5().unwrap(),
                seed6: th19.rand_seed6().unwrap(),
                seed7: th19.rand_seed7().unwrap(),
                seed8: th19.rand_seed8().unwrap(),
            });
        } else {
            let (init, delay_remainings) = session.recv_init_round().unwrap();
            println!("delay_remainings: {}", delay_remainings);
            th19.set_rand_seed1(init.seed1).unwrap();
            th19.set_rand_seed2(init.seed2).unwrap();
            th19.set_rand_seed3(init.seed3).unwrap();
            th19.set_rand_seed4(init.seed4).unwrap();
            th19.set_rand_seed5(init.seed5).unwrap();
            th19.set_rand_seed6(init.seed6).unwrap();
            th19.set_rand_seed7(init.seed7).unwrap();
            th19.set_rand_seed8(init.seed8).unwrap();
        }
    }
}

extern "thiscall" fn on_loaded_game_settings(this: *const c_void, arg1: u32) -> u32 {
    let state = state_mut();
    if let Some(match_initial) = &state.match_initial {
        state
            .th19
            .put_game_settings_in_game(&match_initial.game_settings)
            .unwrap();
    }

    (props().old_fn_from_13f9d0_0446)(this, arg1)
}

#[no_mangle]
pub extern "stdcall" fn DllMain(_inst_dll: HINSTANCE, reason: u32, _reserved: u32) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        if cfg!(debug_assertions) {
            let _ = unsafe { AllocConsole() };
            std::env::set_var("RUST_BACKTRACE", "1");
        }
        let mut th19 = Th19::new_hooked_process("th19.exe").unwrap();
        let (old_on_input_players, apply_hook_on_input_players) =
            th19.hook_on_input_players(on_input_players);
        let (old_on_input_menu, apply_hook_on_input_menu) = th19.hook_on_input_menu(on_input_menu);
        let (old_fn_from_11f870_034c, apply_hook_11f870_034c) =
            th19.hook_11f870_034c(on_round_over);
        let (old_fn_from_13f9d0_0446, apply_hook_13f9d0_0446) =
            th19.hook_13f9d0_0446(on_loaded_game_settings);
        let (session_sender, session_receiver) = mpsc::channel();
        init_interprocess(session_sender);
        unsafe {
            PROPS = Some(Props {
                old_on_input_players,
                old_on_input_menu,
                old_fn_from_11f870_034c,
                old_fn_from_13f9d0_0446,
                session_receiver,
            })
        };
        unsafe {
            STATE = Some(State {
                th19,
                state: 0x00,
                session: None,
                match_initial: None,
            })
        };
        let th19 = &mut state_mut().th19;
        apply_hook_on_input_players(th19);
        apply_hook_on_input_menu(th19);
        apply_hook_11f870_034c(th19);
        apply_hook_13f9d0_0446(th19);
    }
    true
}
