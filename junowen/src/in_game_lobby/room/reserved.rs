use std::ffi::c_void;

use junowen_lib::{InputValue, Th19};

use crate::{
    file::SettingsRepo,
    signaling::waiting_for_match::{
        WaitingForMatch, WaitingForOpponent, WaitingForOpponentInReservedRoom,
        WaitingForSpectatorHost, WaitingForSpectatorHostInReservedRoom, WaitingInRoom,
    },
    TOKIO_RUNTIME,
};

use super::{
    super::common_menu::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult},
    on_render_texts,
};

fn make_menu(room_name: String) -> (u8, CommonMenu) {
    let menu = Menu::new(
        "Reserved Room",
        None,
        vec![
            MenuItem::sub_menu(
                "Enter as a Player",
                Some(0),
                Menu::new(
                    "Reserved Room",
                    Some(1),
                    vec![MenuItem::plain("Leave", 1, true)],
                    0,
                ),
            ),
            MenuItem::sub_menu(
                "Enter as a Spectator",
                Some(3),
                Menu::new(
                    "Reserved Room",
                    Some(1),
                    vec![MenuItem::plain("Leave", 1, true)],
                    0,
                ),
            ),
            MenuItem::text_input("Change Room Name", 4, "Room name", room_name),
        ],
        0,
    );
    (1, CommonMenu::new(false, 240 + 56, menu))
}

pub struct ReservedRoom {
    menu_id: u8,
    menu: CommonMenu,
    room_name: String,
}

impl ReservedRoom {
    pub fn new() -> Self {
        Self {
            menu_id: 0,
            menu: CommonMenu::new(false, 0, Menu::new("", None, vec![], 0)),
            room_name: String::new(),
        }
    }

    pub fn on_input_menu(
        &mut self,
        settings_repo: &SettingsRepo,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        waiting: &mut Option<WaitingForMatch>,
    ) -> Option<LobbyScene> {
        if self.menu_id == 0 {
            self.room_name = TOKIO_RUNTIME.block_on(settings_repo.reserved_room_name(th19));
            (self.menu_id, self.menu) = make_menu(self.room_name.to_owned());
        }
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
            OnMenuInputResult::None => None,
            OnMenuInputResult::Cancel => {
                *waiting = None;
                (self.menu_id, self.menu) = make_menu(self.room_name.to_owned());
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(action) => match action.id() {
                0 => {
                    *waiting = Some(WaitingForMatch::Opponent(WaitingForOpponent::ReservedRoom(
                        WaitingForOpponentInReservedRoom::new(self.room_name.to_owned()),
                    )));
                    None
                }
                1 => {
                    *waiting = None;
                    (self.menu_id, self.menu) = make_menu(self.room_name.to_owned());
                    None
                }
                3 => {
                    *waiting = Some(WaitingForMatch::SpectatorHost(
                        WaitingForSpectatorHost::ReservedRoom(
                            WaitingForSpectatorHostInReservedRoom::new(self.room_name.to_owned()),
                        ),
                    ));
                    None
                }
                4 => {
                    let new_room_name = action.value().unwrap().to_owned();
                    self.room_name = new_room_name.clone();
                    TOKIO_RUNTIME.block_on(settings_repo.set_reserved_room_name(new_room_name));
                    None
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn on_render_texts<T>(
        &self,
        mut waiting: Option<&WaitingInRoom<T>>,
        th19: &Th19,
        text_renderer: *const c_void,
    ) {
        if !self.menu.menu().decided() {
            waiting = None;
        }
        on_render_texts(&self.menu, waiting, &self.room_name, th19, text_renderer);
    }
}
