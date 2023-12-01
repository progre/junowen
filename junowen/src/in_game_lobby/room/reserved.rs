use std::ffi::c_void;

use junowen_lib::{InputValue, Th19};

use crate::signaling::waiting_for_match::{
    WaitingForMatch, WaitingForOpponent, WaitingForOpponentInReservedRoom, WaitingForSpectatorHost,
    WaitingForSpectatorHostInReservedRoom, WaitingInRoom,
};

use super::{
    super::common_menu::{
        CommonMenu, LobbyScene, MenuAction, MenuDefine, MenuItem, OnMenuInputResult,
    },
    on_render_texts,
};

fn make_enter_menu() -> (u8, CommonMenu) {
    let items = vec![
        MenuItem::new("Enter as Player", MenuAction::Action(0, true).into()),
        MenuItem::new("Enter as Spectator", MenuAction::Action(3, true).into()),
        // MenuItem::new("Change Room Name", MenuAction::Action(2, true).into()),
    ];
    (
        0,
        CommonMenu::new("Reserved Room", false, 240 + 56, MenuDefine::new(0, items)),
    )
}

fn make_leave_menu() -> (u8, CommonMenu) {
    let items = vec![MenuItem::new("Leave", MenuAction::Action(1, true).into())];
    (
        1,
        CommonMenu::new("Reserved Room", false, 240 + 56, MenuDefine::new(0, items)),
    )
}

pub struct ReservedRoom {
    menu_id: u8,
    menu: CommonMenu,
}

impl ReservedRoom {
    pub fn new() -> Self {
        let (menu_id, menu) = make_enter_menu();
        Self { menu_id, menu }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        waiting: &mut Option<WaitingForMatch>,
    ) -> Option<LobbyScene> {
        match waiting {
            Some(WaitingForMatch::Opponent(WaitingForOpponent::ReservedRoom(waiting))) => {
                waiting.recv();
            }
            Some(WaitingForMatch::SpectatorHost(WaitingForSpectatorHost::ReservedRoom(
                waiting,
            ))) => {
                waiting.recv();
            }
            _ => {
                *waiting = None;
            }
        }

        match self.menu.on_input_menu(current_input, prev_input, th19) {
            OnMenuInputResult::None => {
                if waiting.is_none() {
                    if self.menu_id != 0 {
                        (self.menu_id, self.menu) = make_enter_menu();
                    }
                } else {
                    //
                    if self.menu_id != 1 {
                        (self.menu_id, self.menu) = make_leave_menu();
                    }
                }
                None
            }
            OnMenuInputResult::Cancel => {
                if matches!(waiting, Some(WaitingForMatch::SpectatorHost(_))) {
                    *waiting = None;
                    (self.menu_id, self.menu) = make_enter_menu();
                }
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(MenuAction::Action(action, _)) => match action {
                0 => {
                    *waiting = Some(WaitingForMatch::Opponent(WaitingForOpponent::ReservedRoom(
                        WaitingForOpponentInReservedRoom::new(
                            th19.online_vs_mode().room_name().to_owned(),
                        ),
                    )));
                    (self.menu_id, self.menu) = make_leave_menu();
                    None
                }
                1 => {
                    *waiting = None;
                    (self.menu_id, self.menu) = make_enter_menu();
                    None
                }
                3 => {
                    *waiting = Some(WaitingForMatch::SpectatorHost(
                        WaitingForSpectatorHost::ReservedRoom(
                            WaitingForSpectatorHostInReservedRoom::new(
                                th19.online_vs_mode().room_name().to_owned(),
                            ),
                        ),
                    ));
                    (self.menu_id, self.menu) = make_leave_menu();
                    None
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn on_render_texts<T>(
        &self,
        waiting: Option<&WaitingInRoom<T>>,
        th19: &Th19,
        text_renderer: *const c_void,
    ) {
        on_render_texts(&self.menu, waiting, th19, text_renderer);
    }
}
