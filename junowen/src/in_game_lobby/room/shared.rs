use std::ffi::c_void;

use junowen_lib::{InputValue, Th19};

use crate::signaling::waiting_for_match::WaitingForOpponentInSharedRoom;

use super::{
    super::common_menu::{
        CommonMenu, LobbyScene, MenuAction, MenuDefine, MenuItem, OnMenuInputResult,
    },
    on_render_texts,
};

fn make_enter_menu() -> (u8, CommonMenu) {
    let items = vec![
        MenuItem::new("Enter", MenuAction::Action(0, true).into()),
        // MenuItem::new("Change Room Name", MenuAction::Action(2, true).into()),
    ];
    (
        0,
        CommonMenu::new("Shared Room", false, 240 + 56, MenuDefine::new(0, items)),
    )
}

fn make_leave_menu() -> (u8, CommonMenu) {
    let items = vec![MenuItem::new("Leave", MenuAction::Action(1, true).into())];
    (
        1,
        CommonMenu::new("Shared Room", false, 240 + 56, MenuDefine::new(0, items)),
    )
}

pub struct SharedRoom {
    menu_id: u8,
    menu: CommonMenu,
}

impl SharedRoom {
    pub fn new() -> Self {
        let (menu_id, menu) = make_enter_menu();
        Self { menu_id, menu }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        waiting: &mut Option<WaitingForOpponentInSharedRoom>,
    ) -> Option<LobbyScene> {
        if let Some(waiting) = waiting {
            waiting.recv();
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
            OnMenuInputResult::Cancel => Some(LobbyScene::Root),
            OnMenuInputResult::Action(MenuAction::SubScene(_)) => unreachable!(),
            OnMenuInputResult::Action(MenuAction::Action(action, _)) => match action {
                0 => {
                    *waiting = Some(WaitingForOpponentInSharedRoom::new(
                        th19.online_vs_mode().room_name().to_owned(),
                    ));
                    (self.menu_id, self.menu) = make_leave_menu();
                    None
                }
                1 => {
                    *waiting = None;
                    (self.menu_id, self.menu) = make_enter_menu();
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
        on_render_texts(&self.menu, waiting, th19, text_renderer);
    }
}
