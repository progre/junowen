use std::ffi::c_void;

use getset::MutGetters;
use junowen_lib::{InputFlags, InputValue, Th19};

use crate::session::{battle::BattleSession, spectator::SpectatorSessionGuest};

use super::{
    common_menu::{CommonMenu, LobbyScene, MenuAction, MenuDefine, MenuItem, OnMenuInputResult},
    match_standby::{MatchStandby, PureP2pOpponent, PureP2pSpectator},
    pure_p2p_guest::PureP2pGuest,
    pure_p2p_offerer::{pure_p2p_host, pure_p2p_spectator, PureP2pOfferer},
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
                MenuItem::new("Connect as Spectator", LobbyScene::PureP2pSpectator.into()),
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
                th19.menu_input_mut().set_current(InputFlags::PAUSE.into());
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::Action(MenuAction::SubScene(scene)) => Some(scene),
            OnMenuInputResult::Action(MenuAction::Action(..)) => unreachable!(),
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        self.common_menu.on_render_texts(th19, text_renderer);
    }
}

#[derive(MutGetters)]
pub struct Lobby {
    scene: LobbyScene,
    prev_scene: LobbyScene,
    root: Root,
    pure_p2p_host: Option<PureP2pOfferer<BattleSession>>,
    pure_p2p_guest: Option<PureP2pGuest>,
    pure_p2p_spectator: Option<PureP2pOfferer<SpectatorSessionGuest>>,
    prev_input: InputValue,
    #[get_mut = "pub"]
    match_standby: Option<MatchStandby>,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            scene: LobbyScene::Root,
            prev_scene: LobbyScene::Root,
            root: Root::new(),
            match_standby: None,
            pure_p2p_host: None,
            pure_p2p_guest: None,
            pure_p2p_spectator: None,
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
                    self.match_standby = None;
                    self.pure_p2p_host = Some(pure_p2p_host());
                    self.pure_p2p_guest = None;
                    self.pure_p2p_spectator = None;
                }
                let mut session_rx = None;
                let ret = self.pure_p2p_host.as_mut().unwrap().on_input_menu(
                    current_input,
                    self.prev_input,
                    th19,
                    &mut session_rx,
                );
                if let Some(session_rx) = session_rx {
                    self.match_standby = Some(PureP2pOpponent::new(session_rx).into());
                }
                ret
            }
            LobbyScene::PureP2pGuest => {
                if self.pure_p2p_guest.is_none() {
                    self.match_standby = None;
                    self.pure_p2p_guest = Some(PureP2pGuest::new());
                    self.pure_p2p_host = None;
                    self.pure_p2p_spectator = None;
                }
                let mut session_rx = None;
                let ret = self.pure_p2p_guest.as_mut().unwrap().on_input_menu(
                    current_input,
                    self.prev_input,
                    th19,
                    &mut session_rx,
                );
                if let Some(session_rx) = session_rx {
                    self.match_standby = Some(PureP2pOpponent::new(session_rx).into());
                }
                ret
            }
            LobbyScene::PureP2pSpectator => {
                if self.pure_p2p_spectator.is_none() {
                    self.match_standby = None;
                    self.pure_p2p_spectator = Some(pure_p2p_spectator());
                    self.pure_p2p_host = None;
                    self.pure_p2p_guest = None;
                }
                let mut session_rx = None;
                let ret = self.pure_p2p_spectator.as_mut().unwrap().on_input_menu(
                    current_input,
                    self.prev_input,
                    th19,
                    &mut session_rx,
                );
                if let Some(session_rx) = session_rx {
                    self.match_standby = Some(PureP2pSpectator::new(session_rx).into());
                }
                ret
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
            LobbyScene::PureP2pSpectator => self
                .pure_p2p_spectator
                .as_ref()
                .unwrap()
                .on_render_texts(th19, text_renderer),
        }
    }
}
