use std::ffi::c_void;

use junowen_lib::{Th19, structs::input_devices::InputValue};

use crate::{
    TOKIO_RUNTIME, file::SettingsRepo, signaling::waiting_for_match::WaitingForOpponentOnLan,
};

use super::{
    super::common_menu::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult},
    on_render_texts,
};

const OFFLINE_LABEL: &str = "Offline Mode: ON";
const ONLINE_LABEL: &str = "Offline Mode: OFF";

fn make_menu() -> CommonMenu {
    let leave_menu = || Menu::new("LAN", Some(1), vec![MenuItem::plain("Leave", 1, false)], 0);
    let menu = Menu::new(
        "LAN",
        None,
        vec![
            MenuItem::sub_menu("Connect as a Host", Some(0), leave_menu()),
            MenuItem::sub_menu("Connect as a Guest", Some(3), leave_menu()),
            MenuItem::text_input("Change Address", 11, 12, "Address"),
            MenuItem::plain(OFFLINE_LABEL, 20, true),
        ],
        0,
    );
    CommonMenu::new(false, 240 + 56, menu)
}

pub struct Lan {
    menu: CommonMenu,
    enter: bool,
    address: Option<String>,
    offline: Option<bool>,
}

impl Lan {
    pub fn new() -> Self {
        Self {
            menu: make_menu(),
            enter: false,
            address: None,
            offline: None,
        }
    }

    fn address(&self) -> &str {
        self.address.as_ref().unwrap()
    }

    fn offline(&self) -> bool {
        self.offline.unwrap()
    }

    /// STUN の到達性はネットワークによって変わるため、LAN 対戦の待ち時間を避けるかどうかを
    /// 手動で切り替えられるようにする(既定はオフライン=STUN 無効)
    fn set_offline(&mut self, offline: bool) {
        self.offline = Some(offline);
        let label = if offline { OFFLINE_LABEL } else { ONLINE_LABEL };
        self.menu.menu_mut().items_mut()[3].set_label(label);
    }

    pub fn on_input_menu(
        &mut self,
        settings_repo: &SettingsRepo,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        waiting: &mut Option<WaitingForOpponentOnLan>,
    ) -> Option<LobbyScene> {
        if self.address.is_none() {
            self.address = Some(TOKIO_RUNTIME.block_on(settings_repo.lan_address()));
        }
        if self.offline.is_none() {
            let offline = TOKIO_RUNTIME.block_on(settings_repo.lan_offline());
            self.set_offline(offline);
        }
        if waiting.is_none() && self.enter {
            self.enter = false;
            assert!(self.menu.menu_mut().bury());
        }
        if let Some(waiting) = waiting {
            waiting.recv();
        }

        match self.menu.on_input_menu(current_input, prev_input, th19) {
            OnMenuInputResult::None => None,
            OnMenuInputResult::Cancel => {
                self.enter = false;
                *waiting = None;
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(action) => match action.id() {
                0 => {
                    self.enter = true;
                    *waiting = Some(WaitingForOpponentOnLan::new_lan_host(
                        self.address().to_owned(),
                        self.offline(),
                    ));
                    None
                }
                1 => {
                    self.enter = false;
                    *waiting = None;
                    th19.play_sound(th19.sound_manager(), 0x09, 0);
                    self.menu.controller_mut().force_cancel();
                    None
                }
                3 => {
                    self.enter = true;
                    *waiting = Some(WaitingForOpponentOnLan::new_lan_guest(
                        self.address().to_owned(),
                        self.offline(),
                    ));
                    None
                }
                11 => {
                    let address = self.address().to_owned();
                    let MenuItem::TextInput(text_input_item) =
                        self.menu.menu_mut().selected_item_mut()
                    else {
                        unreachable!()
                    };
                    text_input_item.text_input_mut().set_value(address);
                    None
                }
                12 => {
                    let new_address = action.value().unwrap().to_owned();
                    self.address = Some(new_address.clone());
                    TOKIO_RUNTIME.block_on(settings_repo.set_lan_address(new_address));
                    None
                }
                20 => {
                    let offline = !self.offline();
                    self.set_offline(offline);
                    TOKIO_RUNTIME.block_on(settings_repo.set_lan_offline(offline));
                    None
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn on_render_texts(
        &self,
        mut waiting: Option<&WaitingForOpponentOnLan>,
        th19: &Th19,
        text_renderer: &c_void,
    ) {
        let mut address = Some(self.address());
        if !self.menu.menu().decided() {
            waiting = None;
        } else if waiting.is_none() {
            address = None;
        }
        on_render_texts(&self.menu, waiting, "Address", address, th19, text_renderer);
    }
}
