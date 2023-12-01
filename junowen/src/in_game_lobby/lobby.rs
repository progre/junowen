use std::ffi::c_void;

use getset::{Getters, MutGetters};
use junowen_lib::{InputFlags, InputValue, Th19};

use crate::{
    session::{battle::BattleSession, spectator::SpectatorSession},
    signaling::waiting_for_match::{
        WaitingForMatch, WaitingForOpponent, WaitingForOpponentInReservedRoom,
        WaitingForPureP2pOpponent, WaitingForPureP2pSpectatorHost, WaitingForSpectatorHost,
    },
};

use super::{
    common_menu::{
        CommonMenu, LobbyScene, MenuAction, MenuContent, MenuDefine, MenuItem, OnMenuInputResult,
    },
    pure_p2p_guest::PureP2pGuest,
    pure_p2p_offerer::{pure_p2p_host, pure_p2p_spectator, PureP2pOfferer},
    room::{reserved::ReservedRoom, shared::SharedRoom},
};

pub struct Root {
    common_menu: CommonMenu,
}

impl Root {
    pub fn new() -> Self {
        let menu_define = MenuDefine::new(
            0,
            vec![
                MenuItem::new("Shared Room", LobbyScene::SharedRoom.into()),
                MenuItem::new("Reserved Room", LobbyScene::ReservedRoom.into()),
                MenuItem::new(
                    "Pure P2P",
                    MenuContent::SubMenu(MenuDefine::new(
                        0,
                        vec![
                            MenuItem::new("Connect as Host", LobbyScene::PureP2pHost.into()),
                            MenuItem::new("Connect as Guest", LobbyScene::PureP2pGuest.into()),
                            MenuItem::new(
                                "Connect as Spectator",
                                LobbyScene::PureP2pSpectator.into(),
                            ),
                        ],
                    )),
                ),
            ],
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
            OnMenuInputResult::SubScene(scene) => Some(scene),
            OnMenuInputResult::Action(MenuAction::Action(..)) => unreachable!(),
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        self.common_menu.on_render_texts(th19, text_renderer);
    }
}

#[derive(MutGetters, Getters)]
pub struct Lobby {
    scene: LobbyScene,
    prev_scene: LobbyScene,
    root: Root,
    shared_room: SharedRoom,
    reserved_room: ReservedRoom,
    pure_p2p_host: Option<PureP2pOfferer<BattleSession>>,
    pure_p2p_guest: Option<PureP2pGuest>,
    pure_p2p_spectator: Option<PureP2pOfferer<SpectatorSession>>,
    prev_input: InputValue,
    #[getset(get = "pub", get_mut = "pub")]
    waiting_for_match: Option<WaitingForMatch>,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            scene: LobbyScene::Root,
            prev_scene: LobbyScene::Root,
            root: Root::new(),
            waiting_for_match: None,
            shared_room: SharedRoom::new(),
            reserved_room: ReservedRoom::new(),
            pure_p2p_host: None,
            pure_p2p_guest: None,
            pure_p2p_spectator: None,
            prev_input: InputValue::full(),
        }
    }

    pub fn reset_depth(&mut self) {
        // self.scene = LobbyScene::Root;
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
            LobbyScene::SharedRoom => {
                let mut waiting = match self.waiting_for_match.take() {
                    Some(WaitingForMatch::Opponent(WaitingForOpponent::SharedRoom(waiting))) => {
                        Some(waiting)
                    }
                    _ => None,
                };
                let ret = self.shared_room.on_input_menu(
                    current_input,
                    self.prev_input,
                    th19,
                    &mut waiting,
                );
                self.waiting_for_match = waiting
                    .map(WaitingForOpponent::SharedRoom)
                    .map(WaitingForMatch::Opponent);
                ret
            }
            LobbyScene::ReservedRoom => self.reserved_room.on_input_menu(
                current_input,
                self.prev_input,
                th19,
                &mut self.waiting_for_match,
            ),
            LobbyScene::PureP2pHost => {
                if self.pure_p2p_host.is_none() {
                    self.waiting_for_match = None;
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
                    self.waiting_for_match =
                        Some(WaitingForPureP2pOpponent::new(session_rx).into());
                }
                ret
            }
            LobbyScene::PureP2pGuest => {
                if self.pure_p2p_guest.is_none() {
                    self.waiting_for_match = None;
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
                    self.waiting_for_match =
                        Some(WaitingForPureP2pOpponent::new(session_rx).into());
                }
                ret
            }
            LobbyScene::PureP2pSpectator => {
                if self.pure_p2p_spectator.is_none() {
                    self.waiting_for_match = None;
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
                    self.waiting_for_match =
                        Some(WaitingForPureP2pSpectatorHost::new(session_rx).into());
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
            LobbyScene::SharedRoom => {
                let waiting = self.waiting_for_match.as_ref().and_then(|x| match x {
                    WaitingForMatch::Opponent(WaitingForOpponent::SharedRoom(waiting)) => {
                        Some(waiting)
                    }
                    _ => None,
                });
                self.shared_room
                    .on_render_texts(waiting, th19, text_renderer);
            }
            LobbyScene::ReservedRoom => match &self.waiting_for_match {
                Some(WaitingForMatch::Opponent(WaitingForOpponent::ReservedRoom(waiting))) => {
                    self.reserved_room
                        .on_render_texts(Some(waiting), th19, text_renderer);
                }
                Some(WaitingForMatch::SpectatorHost(WaitingForSpectatorHost::ReservedRoom(
                    waiting,
                ))) => {
                    self.reserved_room
                        .on_render_texts(Some(waiting), th19, text_renderer);
                }
                _ => {
                    let none: Option<&WaitingForOpponentInReservedRoom> = None;
                    self.reserved_room
                        .on_render_texts(none, th19, text_renderer);
                }
            },
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
