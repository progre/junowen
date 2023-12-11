use std::ffi::c_void;

use junowen_lib::{InputValue, Th19};

use crate::{
    file::SettingsRepo, in_game_lobby::common_menu::MenuChild,
    signaling::waiting_for_match::WaitingForOpponentInSharedRoom, TOKIO_RUNTIME,
};

use super::{
    super::common_menu::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult},
    on_render_texts,
};

fn make_enter_menu() -> (u8, CommonMenu) {
    let items = vec![
        MenuItem::plain("Enter the Room", 0, true),
        MenuItem::text_input("Change Room Name", 11, 12, "Room name"),
    ];
    (
        1,
        CommonMenu::new(false, 240 + 56, Menu::new("Shared Room", None, items, 0)),
    )
}

fn make_leave_menu() -> (u8, CommonMenu) {
    let items = vec![MenuItem::plain("Leave", 1, true)];
    (
        2,
        CommonMenu::new(false, 240 + 56, Menu::new("Shared Room", None, items, 0)),
    )
}

pub struct SharedRoom {
    menu_id: u8,
    menu: CommonMenu,
    room_name: String,
}

impl SharedRoom {
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
        waiting: &mut Option<WaitingForOpponentInSharedRoom>,
    ) -> Option<LobbyScene> {
        if self.menu_id == 0 {
            self.room_name = TOKIO_RUNTIME.block_on(settings_repo.shared_room_name(th19));
            (self.menu_id, self.menu) = make_enter_menu();
        }

        if let Some(waiting) = waiting {
            waiting.recv();
        }
        match self.menu.on_input_menu(current_input, prev_input, th19) {
            OnMenuInputResult::None => {
                if waiting.is_none() {
                    if self.menu_id != 1 {
                        (self.menu_id, self.menu) = make_enter_menu();
                    }
                } else {
                    //
                    if self.menu_id != 2 {
                        (self.menu_id, self.menu) = make_leave_menu();
                    }
                }
                None
            }
            OnMenuInputResult::Cancel => Some(LobbyScene::Root),
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(action) => match action.id() {
                0 => {
                    let room_name = self.room_name.to_owned();
                    *waiting = Some(WaitingForOpponentInSharedRoom::new(room_name));
                    (self.menu_id, self.menu) = make_leave_menu();
                    None
                }
                1 => {
                    *waiting = None;
                    (self.menu_id, self.menu) = make_enter_menu();
                    None
                }
                11 => {
                    let Some(MenuChild::TextInput(text_input)) =
                        self.menu.menu_mut().selected_item_mut().child_mut()
                    else {
                        unreachable!()
                    };
                    text_input.set_value(self.room_name.to_owned());
                    None
                }
                12 => {
                    let new_room_name = action.value().unwrap().to_owned();
                    self.room_name = new_room_name.clone();
                    TOKIO_RUNTIME.block_on(settings_repo.set_shared_room_name(new_room_name));
                    None
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn on_render_texts(
        &self,
        waiting: Option<&WaitingForOpponentInSharedRoom>,
        th19: &Th19,
        text_renderer: *const c_void,
    ) {
        on_render_texts(
            &self.menu,
            waiting,
            Some(&self.room_name),
            th19,
            text_renderer,
        );
    }
}
