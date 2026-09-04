use std::ffi::c_void;

use clipboard_win::set_clipboard_string;
use junowen_lib::{
    Th19,
    connection::signaling::{
        SignalingCodeType, socket::async_read_write_socket::SignalingServerMessage,
    },
    structs::input_devices::InputValue,
};
use rust_i18n::t;
use tokio::sync::mpsc;

use crate::session::battle::BattleSession;

use super::{
    super::signaling::Signaling,
    common_menu::{CommonMenu, LobbyScene, Menu, MenuItem, OnMenuInputResult},
    helper::signaling_code_line_count,
    overlay::OverlayText,
    signaling_code::paste_signaling_code,
    text_layout::TextLayout,
};

pub struct PureP2pGuest {
    common_menu: CommonMenu,
    signaling: Signaling,
    session_rx: Option<mpsc::Receiver<BattleSession>>,
    offer: Option<String>,
    answer_generated: bool,
    error_received: bool,
}

impl PureP2pGuest {
    pub fn new() -> Self {
        let (session_tx, session_rx) = mpsc::channel(1);
        Self {
            common_menu: CommonMenu::new(
                false,
                840,
                Menu::new(
                    "Connect as a Guest",
                    None,
                    vec![MenuItem::plain("Press SHOT to Paste", 0, false)],
                    0,
                ),
            ),
            signaling: Signaling::new(session_tx, |conn, dc| BattleSession::new(conn, dc, false)),
            session_rx: Some(session_rx),
            offer: None,
            answer_generated: false,
            error_received: false,
        }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        session_rx: &mut Option<mpsc::Receiver<BattleSession>>,
    ) -> Option<LobbyScene> {
        self.signaling.recv();
        if self.signaling.connected() {
            self.reset();
        }
        if !self.answer_generated {
            if let Some(answer) = self.signaling.answer() {
                self.answer_generated = true;
                set_clipboard_string(&SignalingCodeType::BattleAnswer.to_string(answer)).unwrap();
                self.common_menu = CommonMenu::new(
                    false,
                    840,
                    Menu::new(
                        self.common_menu.root_title(),
                        None,
                        vec![MenuItem::plain("Press SHOT to Copy again", 1, true)],
                        0,
                    ),
                )
            }
        }
        if !self.error_received && self.signaling.error().is_some() {
            self.error_received = true;
            self.common_menu = CommonMenu::new(
                false,
                0,
                Menu::new(self.common_menu.root_title(), None, vec![], 0),
            )
        }
        match self
            .common_menu
            .on_input_menu(current_input, prev_input, th19)
        {
            OnMenuInputResult::None => None,
            OnMenuInputResult::Cancel => {
                self.reset();
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(action) => {
                match action.id() {
                    0 => {
                        let offer = paste_signaling_code(th19, SignalingCodeType::BattleOffer)?;
                        self.offer = Some(SignalingCodeType::BattleOffer.to_string(&offer));
                        self.signaling
                            .msg_tx_mut()
                            .take()
                            .unwrap()
                            .send(SignalingServerMessage::RequestAnswer(offer))
                            .unwrap();
                        *session_rx = self.session_rx.take();
                        self.common_menu = CommonMenu::new(
                            false,
                            0,
                            Menu::new(self.common_menu.root_title(), None, vec![], 0),
                        )
                    }
                    1 => {
                        set_clipboard_string(
                            &SignalingCodeType::BattleAnswer
                                .to_string(self.signaling.answer().as_ref().unwrap()),
                        )
                        .unwrap();
                        self.error_received = true;
                    }
                    _ => unreachable!(),
                }
                None
            }
        }
    }

    fn layout(&self) -> TextLayout {
        let mut layout = TextLayout::default();
        let mut line = 0;
        'a: {
            layout.push_text(line, t!("pure_p2p.guest_opponent_code"));
            line += 2;
            let Some(offer) = self.offer.as_ref() else {
                break 'a;
            };
            let offer_line_count = signaling_code_line_count(offer);
            layout.push_code(line, offer.to_owned());
            line += offer_line_count + 1;
            layout.push_text(line, t!("pure_p2p.your_code"));
            let Some(answer) = &self.signaling.answer() else {
                break 'a;
            };
            let answer = SignalingCodeType::BattleAnswer.to_string(answer);
            let answer_line_count = signaling_code_line_count(&answer);
            line += 2;
            layout.push_code(line, answer);
            line += answer_line_count + 1;
            layout.push_text(line, t!("pure_p2p.copied_to_clipboard"));
            layout.push_text(line + 1, t!("pure_p2p.guest_share"));
            line += 3;
            layout.push_text(line, t!("pure_p2p.guest_waiting"));
        }
        if let Some(err) = self.signaling.error() {
            line += 2;
            layout.push_text(line, err.to_string());
        }
        layout
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: &c_void) {
        self.common_menu.on_render_texts(th19, text_renderer);
        self.layout().render_codes(th19, text_renderer);
    }

    pub fn overlay_texts(&self) -> Vec<OverlayText> {
        self.layout().into_overlay_texts()
    }

    fn reset(&mut self) {
        self.error_received = false;
        *self = Self::new();
    }
}
