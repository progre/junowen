mod menu;

use std::ffi::c_void;

use junowen_lib::{Th19, structs::input_devices::InputValue};
use rust_i18n::t;

use crate::{
    TOKIO_RUNTIME, file::SettingsRepo,
    signaling::waiting_for_match::WaitingForOpponentOverTcpSignaling,
};

use self::menu::{
    ADDRESS_INPUT_ACTION_ID, CHANGE_ADDRESS_ACTION_ID, GUEST_ACTION_ID, HOST_ACTION_ID,
    OFFLINE_ACTION_ID, OFFLINE_ITEM_INDEX, TcpSignalingMenu,
};

use super::{
    super::common_menu::{LobbyScene, MenuItem, OnMenuInputResult},
    on_render_texts,
};

pub struct TcpSignaling {
    menu: TcpSignalingMenu,
    address: Option<String>,
}

impl TcpSignaling {
    pub fn new() -> Self {
        Self {
            menu: TcpSignalingMenu::new(),
            address: None,
        }
    }

    fn address(&self) -> &str {
        self.address.as_ref().unwrap()
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
        if self.menu.offline().is_none() {
            let offline = TOKIO_RUNTIME.block_on(settings_repo.tcp_signaling_offline());
            self.menu.set_offline(offline);
        }
        if waiting.is_none() && self.menu.has_role() {
            self.menu.change_to_idle();
        }
        if let Some(waiting) = waiting {
            waiting.recv();
        }

        match self
            .menu
            .common_menu_mut()
            .on_input_menu(current_input, prev_input, th19)
        {
            OnMenuInputResult::None => None,
            OnMenuInputResult::Cancel => Some(LobbyScene::Root),
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(action) => match action.id() {
                HOST_ACTION_ID => {
                    if !self.menu.has_role() {
                        *waiting =
                            Some(WaitingForOpponentOverTcpSignaling::new_tcp_signaling_host(
                                self.address().to_owned(),
                                self.menu.offline().unwrap(),
                            ));
                        self.menu.change_to_host();
                    } else {
                        *waiting = None;
                        self.menu.change_to_idle();
                    }
                    None
                }
                GUEST_ACTION_ID => {
                    if !self.menu.has_role() {
                        *waiting =
                            Some(WaitingForOpponentOverTcpSignaling::new_tcp_signaling_guest(
                                self.address().to_owned(),
                                self.menu.offline().unwrap(),
                            ));
                        self.menu.change_to_guest();
                    } else {
                        *waiting = None;
                        self.menu.change_to_idle();
                    }
                    None
                }
                CHANGE_ADDRESS_ACTION_ID => {
                    let address = self.address().to_owned();
                    let MenuItem::TextInput(text_input_item) =
                        self.menu.common_menu_mut().menu_mut().selected_item_mut()
                    else {
                        unreachable!()
                    };
                    text_input_item.text_input_mut().set_value(address);
                    None
                }
                ADDRESS_INPUT_ACTION_ID => {
                    let new_address = action.value().unwrap().to_owned();
                    self.address = Some(new_address.clone());
                    TOKIO_RUNTIME.block_on(settings_repo.set_tcp_signaling_address(new_address));
                    None
                }
                OFFLINE_ACTION_ID => {
                    let offline = !self.menu.offline().unwrap();
                    self.menu.set_offline(offline);
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
            self.menu.common_menu(),
            waiting,
            "Address",
            Some(self.address()),
            th19,
            text_renderer,
        );
    }

    pub fn text(&self, waiting: bool) -> String {
        if waiting {
            t!("lobby.tcp_signaling_waiting").into()
        } else if self.menu.common_menu().menu().cursor() == OFFLINE_ITEM_INDEX {
            t!("lobby.tcp_signaling_offline_mode").into()
        } else {
            String::new()
        }
    }
}
