use std::ffi::c_void;

use clipboard_win::{get_clipboard_string, set_clipboard_string};
use junowen_lib::{
    connection::signaling::{
        parse_signaling_code, socket::async_read_write_socket::SignalingServerMessage,
        SignalingCodeType,
    },
    InputValue, Th19,
};
use tokio::sync::mpsc;

use crate::session::battle::BattleSession;

use super::{
    super::signaling::Signaling,
    common_menu::{CommonMenu, LobbyScene, MenuDefine, MenuItem, OnMenuInputResult},
    helper::{render_small_text_line, render_text_line},
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
                "Connect as a Guest",
                false,
                840,
                MenuDefine::new(
                    0,
                    vec![MenuItem::simple_action("Press SHOT to Paste", 0, false)],
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
                    self.common_menu.root_label(),
                    false,
                    840,
                    MenuDefine::new(
                        0,
                        vec![MenuItem::simple_action("Press SHOT to Copy again", 1, true)],
                    ),
                )
            }
        }
        if !self.error_received && self.signaling.error().is_some() {
            self.error_received = true;
            self.common_menu = CommonMenu::new(
                self.common_menu.root_label(),
                false,
                0,
                MenuDefine::new(0, vec![]),
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
                        let Ok(ok) = get_clipboard_string() else {
                            th19.play_sound(th19.sound_manager(), 0x10, 0);
                            return None;
                        };
                        let Ok((SignalingCodeType::BattleOffer, offer)) = parse_signaling_code(&ok)
                        else {
                            th19.play_sound(th19.sound_manager(), 0x10, 0);
                            return None;
                        };
                        th19.play_sound(th19.sound_manager(), 0x07, 0);
                        self.offer = Some(SignalingCodeType::BattleOffer.to_string(&offer));
                        self.signaling
                            .msg_tx_mut()
                            .take()
                            .unwrap()
                            .send(SignalingServerMessage::RequestAnswer(offer))
                            .unwrap();
                        *session_rx = self.session_rx.take();
                        self.common_menu = CommonMenu::new(
                            self.common_menu.root_label(),
                            false,
                            0,
                            MenuDefine::new(0, vec![]),
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

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        self.common_menu.on_render_texts(th19, text_renderer);

        let mut line = 0;
        'a: {
            render_text_line(th19, text_renderer, line, b"Host's signaling code:");
            line += 2;
            let Some(offer) = self.offer.as_ref() else {
                break 'a;
            };
            let chunks = offer.as_bytes().chunks(100);
            let offer_len = (chunks.len() as f64 / 2.0).ceil() as u32;
            chunks.enumerate().for_each(|(i, chunk)| {
                render_small_text_line(th19, text_renderer, line * 2 + i as u32, chunk);
            });
            line += offer_len + 1;
            render_text_line(th19, text_renderer, line, b"Your signaling code:");
            let Some(answer) = &self.signaling.answer() else {
                break 'a;
            };
            let answer = SignalingCodeType::BattleAnswer.to_string(answer);
            let chunks = answer.as_bytes().chunks(100);
            let answer_len = (chunks.len() as f64 / 2.0).ceil() as u32;
            line += 2;
            chunks.enumerate().for_each(|(i, chunk)| {
                render_small_text_line(th19, text_renderer, line * 2 + i as u32, chunk);
            });
            line += answer_len + 1;
            render_text_line(th19, text_renderer, line, b"It was copied to Clipboard.");
            render_text_line(
                th19,
                text_renderer,
                line + 1,
                b"Share your signaling code with host.",
            );
            line += 3;
            render_text_line(th19, text_renderer, line, b"Waiting for host to connect...");
        }
        if let Some(err) = self.signaling.error() {
            line += 2;
            render_text_line(th19, text_renderer, line, err.to_string().as_bytes());
        }
    }

    fn reset(&mut self) {
        self.error_received = false;
        *self = Self::new();
    }
}
