use std::ffi::c_void;

use clipboard_win::set_clipboard_string;
use junowen_lib::{
    connection::signaling::{
        socket::async_read_write_socket::SignalingServerMessage, CompressedSessionDesc,
    },
    InputValue, Th19,
};
use tokio::sync::mpsc;
use tracing::trace;

use crate::session::Session;

use super::{
    common_menu::{
        CommonMenu, LobbyScene, MenuAction, MenuContent, MenuDefine, MenuItem, OnMenuInputResult,
    },
    helper::{render_small_text_line, render_text_line},
    signaling::Signaling,
};

pub struct PureP2pHost {
    common_menu: CommonMenu,
    signaling: Signaling,
    session_tx: mpsc::Sender<Session>,
    answer: Option<CompressedSessionDesc>,
    /// 0: require generate, 1: copied, 2: already copied
    copy_state: u8,
}

impl PureP2pHost {
    pub fn new(session_tx: mpsc::Sender<Session>) -> Self {
        Self {
            common_menu: CommonMenu::new(
                "Ju.N.Owen",
                false,
                720,
                MenuDefine::new(
                    2,
                    vec![
                        MenuItem::new("Regenerate", MenuContent::Action(MenuAction::Action(0))),
                        MenuItem::new("Copy your code", MenuContent::Action(MenuAction::Action(1))),
                        MenuItem::new(
                            "Paste guest's code",
                            MenuContent::Action(MenuAction::Action(2)),
                        ),
                    ],
                ),
            ),
            signaling: Signaling::new(session_tx.clone()),
            session_tx,
            answer: None,
            copy_state: 0,
        }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
    ) -> Option<LobbyScene> {
        self.signaling.recv();
        if self.signaling.connected() {
            self.reset();
        }
        if self.copy_state == 0 {
            if let Some(offer) = self.signaling.offer() {
                trace!("copied");
                set_clipboard_string(&offer.0).unwrap();
                self.copy_state = 1;
            }
        }
        match self
            .common_menu
            .on_input_menu(current_input, prev_input, th19)
        {
            OnMenuInputResult::None => None,
            OnMenuInputResult::Cancel => {
                self.copy_state = 2;
                if self.answer.is_some() || self.signaling.error().is_some() {
                    self.reset();
                }
                Some(LobbyScene::Root)
            }
            OnMenuInputResult::Action(MenuAction::SubScene(scene)) => Some(scene),
            OnMenuInputResult::Action(MenuAction::Action(action)) => {
                if action == 0 {
                    self.reset();
                }
                if action == 1 {
                    set_clipboard_string(&self.signaling.offer().as_ref().unwrap().0).unwrap();
                    self.copy_state = 1;
                }
                if action == 2 {
                    if let Ok(ok) = clipboard_win::get_clipboard_string() {
                        self.answer = Some(CompressedSessionDesc(ok.clone()));
                        self.signaling
                            .msg_tx_mut()
                            .take()
                            .unwrap()
                            .send(SignalingServerMessage::SetAnswerDesc(
                                CompressedSessionDesc(ok),
                            ))
                            .unwrap();
                        self.common_menu =
                            CommonMenu::new("Ju.N.Owen", false, 720, MenuDefine::new(0, vec![]))
                    }
                }
                None
            }
        }
    }

    pub fn on_render_texts(&self, th19: &Th19, text_renderer: *const c_void) {
        self.common_menu.on_render_texts(th19, text_renderer);

        let mut line = 0;
        'a: {
            let Some(offer) = &self.signaling.offer() else {
                render_text_line(th19, text_renderer, 0, b"Preparing...");
                break 'a;
            };
            render_text_line(th19, text_renderer, line, b"Your signaling code:");
            line += 2;
            let chunks = offer.0.as_bytes().chunks(100);
            let offer_len = (chunks.len() as f64 / 2.0).ceil() as u32;
            chunks.enumerate().for_each(|(i, chunk)| {
                render_small_text_line(th19, text_renderer, line * 2 + i as u32, chunk);
            });
            line += offer_len + 1;
            if self.copy_state == 1 {
                render_text_line(th19, text_renderer, line, b"It was copied to Clipboard.");
                render_text_line(
                    th19,
                    text_renderer,
                    line + 1,
                    b"Share your signaling code with guest.",
                );
            }
            line += 3;
            render_text_line(th19, text_renderer, line, b"Guest's signaling code:");
            let Some(answer) = &self.answer else {
                break 'a;
            };
            let chunks = answer.0.as_bytes().chunks(100);
            let answer_len = (chunks.len() as f64 / 2.0).ceil() as u32;
            line += 2;
            chunks.enumerate().for_each(|(i, chunk)| {
                render_small_text_line(th19, text_renderer, line * 2 + i as u32, chunk);
            });
            line += answer_len + 1;
            render_text_line(
                th19,
                text_renderer,
                line,
                b"Waiting for guest to connect...",
            );
        }
        if let Some(err) = self.signaling.error() {
            line += 1;
            render_text_line(th19, text_renderer, line, err.to_string().as_bytes());
        }
    }

    fn reset(&mut self) {
        *self = Self::new(self.session_tx.clone());
    }
}
