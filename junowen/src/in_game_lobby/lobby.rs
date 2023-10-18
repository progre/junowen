use std::ffi::c_void;

use junowen_lib::{InputFlags, InputValue, Th19};
use tokio::sync::mpsc;

use crate::session::Session;

use super::{
    common_menu::{CommonMenu, LobbyScene, MenuAction, MenuDefine, MenuItem, OnMenuInputResult},
    pure_p2p_guest::PureP2pGuest,
    pure_p2p_host::PureP2pHost,
};

pub struct Root {
    common_menu: CommonMenu,
}

impl Root {
    pub fn new() -> Self {
        let menu_define = MenuDefine::new(
            // vec![
            //     MenuDefine::new("Room match", vec![]),
            //     MenuDefine::new("Random match", vec![]),
            //     MenuDefine::new("Client-Server", vec![]),
            //     MenuDefine::new(
            //         "Pure P2P",
            0,
            vec![
                MenuItem::new("Connect as Host", LobbyScene::PureP2pHost.into()),
                MenuItem::new("Connect as Guest", LobbyScene::PureP2pGuest.into()),
            ],
            //     ),
            // ],
        );
        Self {
            common_menu: CommonMenu::new("Ju.N.Owen", true, 240, menu_define),
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
                th19.menu_input_mut().set_current(InputFlags::START.into());
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::Action(MenuAction::SubScene(scene)) => Some(scene),
            OnMenuInputResult::Action(MenuAction::Action(_)) => unreachable!(),
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        self.common_menu.on_render_texts(th19, text_renderer);
    }
}

pub struct Lobby {
    scene: LobbyScene,
    prev_scene: LobbyScene,
    root: Root,
    session_tx: mpsc::Sender<Session>,
    pure_p2p_host: Option<PureP2pHost>,
    pure_p2p_guest: Option<PureP2pGuest>,
    prev_input: InputValue,
}

impl Lobby {
    pub fn new(session_tx: mpsc::Sender<Session>) -> Self {
        Self {
            scene: LobbyScene::Root,
            prev_scene: LobbyScene::Root,
            root: Root::new(),
            session_tx,
            pure_p2p_host: None,
            pure_p2p_guest: None,
            prev_input: InputValue::full(),
        }
    }

    pub fn reset_depth(&mut self) {
        self.scene = LobbyScene::Root;
        self.prev_input = InputValue::full();
    }

    pub fn on_input_menu(&mut self, th19: &mut Th19) {
        self.prev_scene = self.scene;
        let current_input = th19.menu_input().current();
        th19.menu_input_mut().set_current(InputValue::empty());

        if let Some(scene) = match self.scene {
            LobbyScene::Root => self
                .root
                .on_input_menu(current_input, self.prev_input, th19),
            LobbyScene::PureP2pHost => {
                if self.pure_p2p_host.is_none() {
                    self.pure_p2p_host = Some(PureP2pHost::new(self.session_tx.clone()));
                    self.pure_p2p_guest = None;
                }
                self.pure_p2p_host.as_mut().unwrap().on_input_menu(
                    current_input,
                    self.prev_input,
                    th19,
                )
            }
            LobbyScene::PureP2pGuest => {
                if self.pure_p2p_guest.is_none() {
                    self.pure_p2p_guest = Some(PureP2pGuest::new(self.session_tx.clone()));
                    self.pure_p2p_host = None;
                }
                self.pure_p2p_guest.as_mut().unwrap().on_input_menu(
                    current_input,
                    self.prev_input,
                    th19,
                )
            }
        } {
            self.scene = scene;
            self.prev_input = InputValue::full();
        } else {
            self.prev_input = current_input;
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        match self.prev_scene {
            LobbyScene::Root => self.root.on_render_texts(th19, text_renderer),
            LobbyScene::PureP2pHost => self
                .pure_p2p_host
                .as_ref()
                .unwrap()
                .on_render_texts(th19, text_renderer),
            LobbyScene::PureP2pGuest => self
                .pure_p2p_guest
                .as_ref()
                .unwrap()
                .on_render_texts(th19, text_renderer),
        }
    }
}
