use std::ffi::c_void;

use junowen_lib::{Th19, structs::input_devices::InputValue};

use crate::{
    session::{battle::BattleSession, spectator::SpectatorSession},
    signaling::waiting_for_match::{
        WaitingForMatch, WaitingForPureP2pOpponent, WaitingForPureP2pSpectatorHost,
    },
};

use super::{
    common_menu::LobbyScene,
    overlay::OverlayText,
    pure_p2p_guest::PureP2pGuest,
    pure_p2p_offerer::{PureP2pOfferer, pure_p2p_host, pure_p2p_spectator},
};

/// Pure P2P の 3 シーン。1 つを開いたら他は捨てる
#[derive(Default)]
pub struct PureP2pScenes {
    host: Option<PureP2pOfferer<BattleSession>>,
    guest: Option<PureP2pGuest>,
    spectator: Option<PureP2pOfferer<SpectatorSession>>,
}

impl PureP2pScenes {
    pub fn on_input_menu(
        &mut self,
        scene: LobbyScene,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        waiting_for_match: &mut Option<WaitingForMatch>,
    ) -> Option<LobbyScene> {
        match scene {
            LobbyScene::PureP2pHost => {
                if self.host.is_none() {
                    *waiting_for_match = None;
                    *self = Self {
                        host: Some(pure_p2p_host()),
                        ..Default::default()
                    };
                }
                let mut session_rx = None;
                let ret = self.host.as_mut().unwrap().on_input_menu(
                    current_input,
                    prev_input,
                    th19,
                    &mut session_rx,
                );
                if let Some(session_rx) = session_rx {
                    *waiting_for_match = Some(WaitingForPureP2pOpponent::new(session_rx).into());
                }
                ret
            }
            LobbyScene::PureP2pGuest => {
                if self.guest.is_none() {
                    *waiting_for_match = None;
                    *self = Self {
                        guest: Some(PureP2pGuest::new()),
                        ..Default::default()
                    };
                }
                let mut session_rx = None;
                let ret = self.guest.as_mut().unwrap().on_input_menu(
                    current_input,
                    prev_input,
                    th19,
                    &mut session_rx,
                );
                if let Some(session_rx) = session_rx {
                    *waiting_for_match = Some(WaitingForPureP2pOpponent::new(session_rx).into());
                }
                ret
            }
            LobbyScene::PureP2pSpectator => {
                if self.spectator.is_none() {
                    *waiting_for_match = None;
                    *self = Self {
                        spectator: Some(pure_p2p_spectator()),
                        ..Default::default()
                    };
                }
                let mut session_rx = None;
                let ret = self.spectator.as_mut().unwrap().on_input_menu(
                    current_input,
                    prev_input,
                    th19,
                    &mut session_rx,
                );
                if let Some(session_rx) = session_rx {
                    *waiting_for_match =
                        Some(WaitingForPureP2pSpectatorHost::new(session_rx).into());
                }
                ret
            }
            _ => unreachable!(),
        }
    }

    pub fn on_render_texts(&self, scene: LobbyScene, th19: &Th19, text_renderer: &c_void) {
        match scene {
            LobbyScene::PureP2pHost => self
                .host
                .as_ref()
                .unwrap()
                .on_render_texts(th19, text_renderer),
            LobbyScene::PureP2pGuest => self
                .guest
                .as_ref()
                .unwrap()
                .on_render_texts(th19, text_renderer),
            LobbyScene::PureP2pSpectator => self
                .spectator
                .as_ref()
                .unwrap()
                .on_render_texts(th19, text_renderer),
            _ => unreachable!(),
        }
    }

    pub fn overlay_texts(&self, scene: LobbyScene) -> Vec<OverlayText> {
        let texts = match scene {
            LobbyScene::PureP2pHost => self.host.as_ref().map(|scene| scene.overlay_texts()),
            LobbyScene::PureP2pGuest => self.guest.as_ref().map(|scene| scene.overlay_texts()),
            LobbyScene::PureP2pSpectator => {
                self.spectator.as_ref().map(|scene| scene.overlay_texts())
            }
            _ => unreachable!(),
        };
        texts.unwrap_or_default()
    }
}
