use std::ffi::c_void;

use clipboard_win::set_clipboard_string;
use junowen_lib::{
    connection::signaling::{
        socket::async_read_write_socket::SignalingServerMessage, CompressedSessionDesc,
    },
    InputValue, Th19,
};
use tokio::sync::mpsc;

use crate::session::battle::BattleSession;

use super::{
    common_menu::{CommonMenu, LobbyScene, MenuAction, MenuDefine, MenuItem, OnMenuInputResult},
    helper::{render_small_text_line, render_text_line},
    signaling::Signaling,
};

pub struct PureP2pGuest {
    common_menu: CommonMenu,
    signaling: Signaling,
    battle_session_tx: mpsc::Sender<BattleSession>,
    offer: Option<CompressedSessionDesc>,
    answer_generated: bool,
    error_received: bool,
}

impl PureP2pGuest {
    pub fn new(battle_session_tx: mpsc::Sender<BattleSession>) -> Self {
        Self {
            common_menu: CommonMenu::new(
                "Ju.N.Owen",
                false,
                840,
                MenuDefine::new(
                    0,
                    vec![MenuItem::new(
                        "Press SHOT to Paste",
                        MenuAction::Action(0, true).into(),
                    )],
                ),
            ),
            signaling: Signaling::new(battle_session_tx.clone(), |conn, dc| {
                BattleSession::new(conn, dc, false)
            }),
            battle_session_tx,
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
    ) -> Option<LobbyScene> {
        self.signaling.recv();
        if self.signaling.connected() {
            self.reset();
        }
        if !self.answer_generated {
            if let Some(answer) = self.signaling.answer() {
                self.answer_generated = true;
                set_clipboard_string(&answer.0).unwrap();
                self.common_menu = CommonMenu::new(
                    "Ju.N.Owen",
                    false,
                    840,
                    MenuDefine::new(
                        0,
                        vec![MenuItem::new(
                            "Press SHOT to Copy again",
                            MenuAction::Action(1, true).into(),
                        )],
                    ),
                )
            }
        }
        if !self.error_received && self.signaling.error().is_some() {
            self.error_received = true;
            self.common_menu = CommonMenu::new("Ju.N.Owen", false, 0, MenuDefine::new(0, vec![]))
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
            OnMenuInputResult::Action(MenuAction::SubScene(_)) => unreachable!(),
            OnMenuInputResult::Action(MenuAction::Action(action, _)) => {
                match action {
                    0 => {
                        if let Ok(ok) = clipboard_win::get_clipboard_string() {
                            let offer = CompressedSessionDesc(ok.clone());
                            self.signaling
                                .msg_tx_mut()
                                .take()
                                .unwrap()
                                .send(SignalingServerMessage::RequestAnswer(offer))
                                .unwrap();
                            self.offer = Some(CompressedSessionDesc(ok));
                            self.common_menu =
                                CommonMenu::new("Ju.N.Owen", false, 0, MenuDefine::new(0, vec![]))
                        }
                    }
                    1 => {
                        set_clipboard_string(&self.signaling.answer().as_ref().unwrap().0).unwrap();
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
            let chunks = offer.0.as_bytes().chunks(100);
            let offer_len = (chunks.len() as f64 / 2.0).ceil() as u32;
            chunks.enumerate().for_each(|(i, chunk)| {
                render_small_text_line(th19, text_renderer, line * 2 + i as u32, chunk);
            });
            line += offer_len + 1;
            render_text_line(th19, text_renderer, line, b"Your signaling code:");
            let Some(answer) = &self.signaling.answer() else {
                break 'a;
            };
            let chunks = answer.0.as_bytes().chunks(100);
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
        *self = Self::new(self.battle_session_tx.clone());
    }
}
