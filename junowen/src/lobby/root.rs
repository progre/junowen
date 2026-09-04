use std::ffi::c_void;

use junowen_lib::{
    Th19,
    structs::input_devices::{InputFlags, InputValue},
};
use rust_i18n::t;

use super::common_menu::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult};

pub struct Root {
    common_menu: CommonMenu,
}

impl Root {
    pub fn new() -> Self {
        let menu = Menu::new(
            "Ju.N.Owen",
            None,
            vec![
                MenuItem::sub_scene("Shared Room", LobbyScene::SharedRoom),
                MenuItem::sub_scene("Reserved Room", LobbyScene::ReservedRoom),
                MenuItem::sub_scene("TCP Signaling", LobbyScene::TcpSignaling),
                MenuItem::sub_menu(
                    "Pure P2P",
                    None,
                    Menu::new(
                        "Pure P2P",
                        None,
                        vec![
                            MenuItem::sub_scene("Connect as a Host", LobbyScene::PureP2pHost),
                            MenuItem::sub_scene("Connect as a Guest", LobbyScene::PureP2pGuest),
                            MenuItem::sub_scene(
                                "Connect as a Spectator",
                                LobbyScene::PureP2pSpectator,
                            ),
                        ],
                        0,
                    ),
                ),
            ],
            0,
        );
        Self {
            common_menu: CommonMenu::new(true, 240, menu),
        }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &mut Th19,
    ) -> Option<LobbyScene> {
        match self
            .common_menu
            .on_input_menu(current_input, prev_input, th19)
        {
            OnMenuInputResult::None => None,
            OnMenuInputResult::Cancel => {
                th19.menu_input_mut().set_current(InputFlags::PAUSE.into());
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::SubScene(scene) => Some(scene),
            OnMenuInputResult::Action(..) => unreachable!(),
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: &c_void) {
        self.common_menu.on_render_texts(th19, text_renderer);
    }

    pub fn text(&self) -> String {
        match self.common_menu.menu().cursor() {
            0 => t!("lobby.shared_room").into(),
            1 => t!("lobby.reserved_room").into(),
            2 => t!("lobby.tcp_signaling").into(),
            3 => {
                if self.common_menu.menu().decided() {
                    "".into()
                } else {
                    t!("lobby.pure_p2p").into()
                }
            }
            _ => unreachable!(),
        }
    }
}
