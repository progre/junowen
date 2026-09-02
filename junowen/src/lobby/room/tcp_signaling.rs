use std::ffi::c_void;

use junowen_lib::{Th19, structs::input_devices::InputValue};

use crate::{
    TOKIO_RUNTIME, file::SettingsRepo,
    signaling::waiting_for_match::WaitingForOpponentOverTcpSignaling,
};

use super::{
    super::common_menu::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult},
    on_render_texts,
};

const HOST_LABEL: &str = "Connect as a Host";
const GUEST_LABEL: &str = "Connect as a Guest";
const LEAVE_LABEL: &str = "Leave";
const OFFLINE_LABEL: &str = "Offline Mode: ON";
const ONLINE_LABEL: &str = "Offline Mode: OFF";

enum Role {
    Host,
    Guest,
}

fn make_menu() -> CommonMenu {
    let menu = Menu::new(
        "TCP Signaling",
        None,
        vec![
            MenuItem::plain(HOST_LABEL, 0, true),
            MenuItem::plain(GUEST_LABEL, 3, true),
            MenuItem::text_input("Change Address", 11, 12, "Address"),
            MenuItem::plain(OFFLINE_LABEL, 20, true),
        ],
        0,
    );
    CommonMenu::new(false, 240 + 56, menu)
}

pub struct TcpSignaling {
    menu: CommonMenu,
    role: Option<Role>,
    address: Option<String>,
    offline: Option<bool>,
}

impl TcpSignaling {
    pub fn new() -> Self {
        Self {
            menu: make_menu(),
            role: None,
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

    /// STUN の到達性はネットワークによって変わるため、待ち時間を避けるかどうかを
    /// 手動で切り替えられるようにする
    fn set_offline(&mut self, offline: bool) {
        self.offline = Some(offline);
        let label = if offline { OFFLINE_LABEL } else { ONLINE_LABEL };
        self.menu.menu_mut().items_mut()[3].set_label(label);
    }

    /// 他の接続方式(Shared Room)と同様に、接続待ち中も他の機能を使えるように
    /// サブメニューへは潜らずフラットなメニュー構成にし、キャンセルでは待機を破棄しない
    fn change_menu_to_idle(&mut self) {
        self.role = None;
        let item = &mut self.menu.menu_mut().items_mut()[0];
        item.set_label(HOST_LABEL);
        item.set_enabled(true);
        let item = &mut self.menu.menu_mut().items_mut()[1];
        item.set_label(GUEST_LABEL);
        item.set_enabled(true);
        let item = &mut self.menu.menu_mut().items_mut()[2];
        item.set_enabled(true);
        let item = &mut self.menu.menu_mut().items_mut()[3];
        item.set_enabled(true);
    }

    fn change_menu_to_host(&mut self) {
        self.role = Some(Role::Host);
        let item = &mut self.menu.menu_mut().items_mut()[0];
        item.set_label(LEAVE_LABEL);
        let item = &mut self.menu.menu_mut().items_mut()[1];
        item.set_enabled(false);
        let item = &mut self.menu.menu_mut().items_mut()[2];
        item.set_enabled(false);
        let item = &mut self.menu.menu_mut().items_mut()[3];
        item.set_enabled(false);
    }

    fn change_menu_to_guest(&mut self) {
        self.role = Some(Role::Guest);
        let item = &mut self.menu.menu_mut().items_mut()[1];
        item.set_label(LEAVE_LABEL);
        let item = &mut self.menu.menu_mut().items_mut()[0];
        item.set_enabled(false);
        let item = &mut self.menu.menu_mut().items_mut()[2];
        item.set_enabled(false);
        let item = &mut self.menu.menu_mut().items_mut()[3];
        item.set_enabled(false);
    }

    pub fn on_input_menu(
        &mut self,
        settings_repo: &SettingsRepo,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        waiting: &mut Option<WaitingForOpponentOverTcpSignaling>,
    ) -> Option<LobbyScene> {
        if self.address.is_none() {
            self.address = Some(TOKIO_RUNTIME.block_on(settings_repo.tcp_signaling_address()));
        }
        if self.offline.is_none() {
            let offline = TOKIO_RUNTIME.block_on(settings_repo.tcp_signaling_offline());
            self.set_offline(offline);
        }
        if waiting.is_none() && self.role.is_some() {
            self.change_menu_to_idle();
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
                    if self.role.is_none() {
                        *waiting =
                            Some(WaitingForOpponentOverTcpSignaling::new_tcp_signaling_host(
                                self.address().to_owned(),
                                self.offline(),
                            ));
                        self.change_menu_to_host();
                    } else {
                        *waiting = None;
                        self.change_menu_to_idle();
                    }
                    None
                }
                3 => {
                    if self.role.is_none() {
                        *waiting =
                            Some(WaitingForOpponentOverTcpSignaling::new_tcp_signaling_guest(
                                self.address().to_owned(),
                                self.offline(),
                            ));
                        self.change_menu_to_guest();
                    } else {
                        *waiting = None;
                        self.change_menu_to_idle();
                    }
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
                    TOKIO_RUNTIME.block_on(settings_repo.set_tcp_signaling_address(new_address));
                    None
                }
                20 => {
                    let offline = !self.offline();
                    self.set_offline(offline);
                    TOKIO_RUNTIME.block_on(settings_repo.set_tcp_signaling_offline(offline));
                    None
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn on_render_texts(
        &self,
        waiting: Option<&WaitingForOpponentOverTcpSignaling>,
        th19: &Th19,
        text_renderer: &c_void,
    ) {
        on_render_texts(
            &self.menu,
            waiting,
            "Address",
            Some(self.address()),
            th19,
            text_renderer,
        );
    }
}
