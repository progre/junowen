mod menu;
mod role;

use std::ffi::c_void;

use clipboard_win::set_clipboard_string;
use junowen_lib::{
    Th19,
    connection::{
        DataChannel, PeerConnection,
        signaling::{SignalingCodeType, socket::async_read_write_socket::SignalingServerMessage},
    },
    structs::input_devices::InputValue,
};
use rust_i18n::t;
use tokio::sync::mpsc;
use tracing::trace;

use super::{
    super::signaling::Signaling,
    common_menu::{CommonMenu, LobbyScene, OnMenuInputResult},
    helper::signaling_code_line_count,
    overlay::OverlayText,
    signaling_code::paste_signaling_code,
    text_layout::TextLayout,
};

use self::menu::{
    COPY_ACTION_ID, PASTE_ACTION_ID, REGENERATE_ACTION_ID, make_empty_menu, make_menu,
};

pub use self::role::{Messages, pure_p2p_host, pure_p2p_spectator};

pub struct PureP2pOfferer<T> {
    offer_type: SignalingCodeType,
    answer_type: SignalingCodeType,
    create_session: fn(PeerConnection, DataChannel) -> T,
    messages: Messages,
    common_menu: CommonMenu,
    signaling: Signaling,
    session_rx: Option<mpsc::Receiver<T>>,
    answer: Option<String>,
    /// 0: require generate, 1: copied, 2: already copied, 3: copied again
    copy_state: u8,
}

impl<T> PureP2pOfferer<T>
where
    T: Send + 'static,
{
    pub fn new(
        offer_type: SignalingCodeType,
        answer_type: SignalingCodeType,
        create_session: fn(PeerConnection, DataChannel) -> T,
        label: &'static str,
        messages: Messages,
    ) -> Self {
        let (session_tx, session_rx) = mpsc::channel(1);
        Self {
            offer_type,
            answer_type,
            create_session,
            messages,
            common_menu: make_menu(label),
            signaling: Signaling::new(session_tx, create_session),
            session_rx: Some(session_rx),
            answer: None,
            copy_state: 0,
        }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        session_rx: &mut Option<mpsc::Receiver<T>>,
    ) -> Option<LobbyScene> {
        self.signaling.recv();
        if self.signaling.connected() {
            self.reset();
        }
        if self.copy_state == 0 {
            if let Some(offer) = self.signaling.offer() {
                trace!("copied");
                set_clipboard_string(&self.offer_type.to_string(offer)).unwrap();
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
            OnMenuInputResult::SubScene(_) => unreachable!(),
            OnMenuInputResult::Action(action) => {
                if action.id() == REGENERATE_ACTION_ID {
                    self.reset();
                }
                if action.id() == COPY_ACTION_ID {
                    set_clipboard_string(
                        &self
                            .offer_type
                            .to_string(self.signaling.offer().as_ref().unwrap()),
                    )
                    .unwrap();
                    self.copy_state = if self.copy_state <= 1 { 1 } else { 3 };
                }
                if action.id() == PASTE_ACTION_ID {
                    let answer = paste_signaling_code(th19, self.answer_type)?;
                    self.answer = Some(self.answer_type.to_string(&answer));
                    self.signaling
                        .msg_tx_mut()
                        .take()
                        .unwrap()
                        .send(SignalingServerMessage::SetAnswerDesc(answer))
                        .unwrap();
                    *session_rx = self.session_rx.take();
                    self.common_menu = make_empty_menu(self.common_menu.root_title());
                }
                None
            }
        }
    }

    fn layout(&self) -> TextLayout {
        let mut layout = TextLayout::default();
        let mut line = 0;
        'a: {
            let Some(offer) = &self.signaling.offer() else {
                layout.push_text(line, t!("pure_p2p.preparing"));
                break 'a;
            };
            let key = if [2, 3].contains(&self.copy_state) {
                "pure_p2p.your_code_already_created"
            } else {
                "pure_p2p.your_code"
            };
            layout.push_text(line, t!(key));
            line += 2;
            let offer = self.offer_type.to_string(offer);
            let offer_line_count = signaling_code_line_count(&offer);
            layout.push_code(line, offer);
            line += offer_line_count + 1;
            if [1, 3].contains(&self.copy_state) {
                layout.push_text(line, t!("pure_p2p.copied_to_clipboard"));
                layout.push_text(line + 1, t!(self.messages.share));
            }
            line += 3;
            layout.push_text(line, t!(self.messages.opponent_code));
            let Some(answer) = &self.answer else {
                break 'a;
            };
            let answer_line_count = signaling_code_line_count(answer);
            line += 2;
            layout.push_code(line, answer.to_owned());
            line += answer_line_count + 1;
            layout.push_text(line, t!(self.messages.waiting));
        }
        if let Some(err) = self.signaling.error() {
            line += 1;
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
        *self = Self::new(
            self.offer_type,
            self.answer_type,
            self.create_session,
            self.common_menu.root_title(),
            self.messages,
        );
    }
}
