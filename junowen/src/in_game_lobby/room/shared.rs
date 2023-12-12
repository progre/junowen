use std::ffi::c_void;

use junowen_lib::{InputValue, Th19};

use crate::{
    file::SettingsRepo, signaling::waiting_for_match::WaitingForOpponentInSharedRoom, TOKIO_RUNTIME,
};

use super::{
    super::common_menu::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult},
    on_render_texts,
};

fn make_menu() -> CommonMenu {
    let items = vec![
        MenuItem::plain("Enter the Room", 0, true),
        MenuItem::text_input("Change Room Name", 11, 12, "Room name"),
    ];
    CommonMenu::new(false, 240 + 56, Menu::new("Shared Room", None, items, 0))
}

pub struct SharedRoom {
    menu: CommonMenu,
    enter: bool,
    room_name: Option<String>,
}

impl SharedRoom {
    pub fn new() -> Self {
        Self {
            menu: make_menu(),
            enter: false,
            room_name: None,
        }
    }

    fn room_name(&self) -> &str {
        self.room_name.as_ref().unwrap()
    }

    fn change_menu_to_enter(&mut self) {
        self.enter = false;
        let item = &mut self.menu.menu_mut().items_mut()[0];
        item.set_label("Enter the Room");
        let item = &mut self.menu.menu_mut().items_mut()[1];
        item.set_enabled(true);
    }
    fn change_menu_to_leave(&mut self) {
        self.enter = true;
        let item = &mut self.menu.menu_mut().items_mut()[0];
        item.set_label("Leave the Room");
        let item = &mut self.menu.menu_mut().items_mut()[1];
        item.set_enabled(false);
    }

    pub fn on_input_menu(
        &mut self,
        settings_repo: &SettingsRepo,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        waiting: &mut Option<WaitingForOpponentInSharedRoom>,
    ) -> Option<LobbyScene> {
        if self.room_name.is_none() {
            self.room_name = Some(TOKIO_RUNTIME.block_on(settings_repo.shared_room_name(th19)));
        }
        if waiting.is_some() != self.enter {
            if waiting.is_some() {
                self.change_menu_to_leave();
            } else {
                self.change_menu_to_enter();
            }
        }

        if let Some(waiting) = waiting {
            waiting.recv();
        }
        match self.menu.on_input_menu(current_input, prev_input, th19) {
            OnMenuInputResult::None => None,
            OnMenuInputResult::Cancel => Some(LobbyScene::Root),
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(action) => match action.id() {
                0 => {
                    if waiting.is_none() {
                        let room_name = self.room_name().to_owned();
                        *waiting = Some(WaitingForOpponentInSharedRoom::new(room_name));
                        self.change_menu_to_leave();
                    } else {
                        *waiting = None;
                        self.change_menu_to_enter();
                    }
                    None
                }
                11 => {
                    let room_name = self.room_name().to_owned();
                    let MenuItem::TextInput(text_input_item) =
                        self.menu.menu_mut().selected_item_mut()
                    else {
                        unreachable!()
                    };
                    text_input_item.text_input_mut().set_value(room_name);
                    None
                }
                12 => {
                    let new_room_name = action.value().unwrap().to_owned();
                    self.room_name = Some(new_room_name.clone());
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
            Some(self.room_name()),
            th19,
            text_renderer,
        );
    }
}
