mod common_menu;
mod helper;
mod overlay;
mod pure_p2p_guest;
mod pure_p2p_offerer;
mod pure_p2p_scenes;
mod room;
mod root;
mod signaling_code;
mod text_layout;
mod title_menu_modifier;

use std::ffi::c_void;

use getset::{Getters, MutGetters};
use junowen_lib::{Th19, structs::input_devices::InputValue};

use crate::{
    file::SettingsRepo,
    signaling::waiting_for_match::{
        WaitingForMatch, WaitingForOpponent, WaitingForOpponentInReservedRoom,
        WaitingForSpectatorHost,
    },
};

use self::{
    common_menu::LobbyScene,
    helper::overlay_description,
    pure_p2p_scenes::PureP2pScenes,
    room::{reserved::ReservedRoom, shared::SharedRoom, tcp_signaling::TcpSignaling},
    root::Root,
};

pub use overlay::{OverlayText, overlay};
pub use title_menu_modifier::TitleMenuModifier;

#[derive(MutGetters, Getters)]
pub struct Lobby {
    settings_repo: SettingsRepo,
    scene: LobbyScene,
    prev_scene: LobbyScene,
    root: Root,
    shared_room: SharedRoom,
    reserved_room: ReservedRoom,
    tcp_signaling: TcpSignaling,
    pure_p2p: PureP2pScenes,
    prev_input: InputValue,
    #[getset(get = "pub", get_mut = "pub")]
    waiting_for_match: Option<WaitingForMatch>,
}

impl Lobby {
    pub fn new(settings_repo: SettingsRepo) -> Self {
        Self {
            settings_repo,
            scene: LobbyScene::Root,
            prev_scene: LobbyScene::Root,
            root: Root::new(),
            waiting_for_match: None,
            shared_room: SharedRoom::new(),
            reserved_room: ReservedRoom::new(),
            tcp_signaling: TcpSignaling::new(),
            pure_p2p: PureP2pScenes::default(),
            prev_input: InputValue::full(),
        }
    }

    pub fn clear_input(&mut self) {
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
                    &self.settings_repo,
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
                &self.settings_repo,
                current_input,
                self.prev_input,
                th19,
                &mut self.waiting_for_match,
            ),
            LobbyScene::TcpSignaling => {
                let mut waiting = match self.waiting_for_match.take() {
                    Some(WaitingForMatch::Opponent(WaitingForOpponent::TcpSignaling(waiting))) => {
                        Some(waiting)
                    }
                    _ => None,
                };
                let ret = self.tcp_signaling.on_input_menu(
                    &self.settings_repo,
                    current_input,
                    self.prev_input,
                    th19,
                    &mut waiting,
                );
                self.waiting_for_match = waiting
                    .map(WaitingForOpponent::TcpSignaling)
                    .map(WaitingForMatch::Opponent);
                ret
            }
            scene @ (LobbyScene::PureP2pHost
            | LobbyScene::PureP2pGuest
            | LobbyScene::PureP2pSpectator) => self.pure_p2p.on_input_menu(
                scene,
                current_input,
                self.prev_input,
                th19,
                &mut self.waiting_for_match,
            ),
        } {
            self.scene = scene;
            self.prev_input = InputValue::full();
        } else {
            self.prev_input = current_input;
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: &c_void) {
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
            LobbyScene::TcpSignaling => {
                let waiting = self.waiting_for_match.as_ref().and_then(|x| match x {
                    WaitingForMatch::Opponent(WaitingForOpponent::TcpSignaling(waiting)) => {
                        Some(waiting)
                    }
                    _ => None,
                });
                self.tcp_signaling
                    .on_render_texts(waiting, th19, text_renderer);
            }
            scene @ (LobbyScene::PureP2pHost
            | LobbyScene::PureP2pGuest
            | LobbyScene::PureP2pSpectator) => {
                self.pure_p2p.on_render_texts(scene, th19, text_renderer)
            }
        }
    }

    pub fn overlay_texts(&self) -> Vec<OverlayText> {
        let waiting = self.waiting_for_match.is_some();
        match self.scene {
            LobbyScene::Root => overlay_description(self.root.text()),
            LobbyScene::SharedRoom => overlay_description(self.shared_room.text(waiting)),
            LobbyScene::ReservedRoom => vec![],
            LobbyScene::TcpSignaling => overlay_description(self.tcp_signaling.text(waiting)),
            scene @ (LobbyScene::PureP2pHost
            | LobbyScene::PureP2pGuest
            | LobbyScene::PureP2pSpectator) => self.pure_p2p.overlay_texts(scene),
        }
    }
}
