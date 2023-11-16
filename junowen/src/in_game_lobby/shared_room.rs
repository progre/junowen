use std::ffi::c_void;

use junowen_lib::{InputValue, RenderingText, Th19};

use super::{
    common_menu::{CommonMenu, LobbyScene, MenuAction, MenuDefine, MenuItem, OnMenuInputResult},
    helper::render_text_line,
    match_standby::SharedRoomOpponent,
};

fn make_enter_menu() -> (u8, CommonMenu) {
    (
        0,
        CommonMenu::new(
            "Shared Room",
            false,
            240 + 56,
            MenuDefine::new(
                0,
                vec![
                    // MenuItem::new("Change Room Name", MenuAction::Action(0, true).into()),
                    // MenuItem::new("Spectate: Disallow", MenuAction::Action(0, true).into()),
                    // MenuItem::new("", MenuAction::Action(0, true).into()),
                    MenuItem::new("Enter", MenuAction::Action(0, true).into()),
                ],
            ),
        ),
    )
}

fn make_leave_menu() -> (u8, CommonMenu) {
    (
        1,
        CommonMenu::new(
            "Shared Room",
            false,
            240 + 56,
            MenuDefine::new(
                0,
                vec![
                    // MenuItem::new("Change Room Name", MenuAction::Action(0, true).into()),
                    // MenuItem::new("Spectate: Disallow", MenuAction::Action(0, true).into()),
                    // MenuItem::new("", MenuAction::Action(0, true).into()),
                    MenuItem::new("Leave", MenuAction::Action(1, true).into()),
                ],
            ),
        ),
    )
}

fn progress_text(progress: f64) -> Vec<u8> {
    const LENGTH: f64 = 20.0;
    let progress = progress * LENGTH % (LENGTH + 1.0);
    let mut text = vec![b'|'; 1];
    text.append(&mut vec![b'#'; progress as usize]);
    let fraction = progress - progress.floor();
    if progress < 20.0 {
        text.push(if fraction < 0.33 {
            b'-'
        } else if fraction < 0.66 {
            b'='
        } else {
            b'*'
        });
    }
    text.append(&mut vec![b'-'; (LENGTH - progress) as usize]);
    text.push(b'|');
    text
}

fn render_room_name(th19: &Th19, text_renderer: *const c_void, room_name: &str) {
    let x = (320 * th19.screen_width().unwrap() / 1280) as f32;
    let y = ((240 - 56) * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text("Room name  :".as_bytes());
    rt.x = x;
    rt.y = y;
    rt.color = 0xffffffff;
    rt.font_type = 0;
    rt.horizontal_align = 1;
    rt.vertical_align = 1;
    th19.render_text(text_renderer, &rt);

    let x = (544 * th19.screen_width().unwrap() / 1280) as f32;
    rt.set_text(room_name.as_bytes());
    rt.color = 0xffffffa0;
    rt.x = x;
    th19.render_text(text_renderer, &rt);
}

fn render_progress(th19: &Th19, text_renderer: *const c_void, text: &[u8]) {
    let x = (640 * th19.screen_width().unwrap() / 1280) as f32;
    let y = ((160 + 32 * 11) * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.x = x;
    rt.y = y;
    rt.color = 0xff000000;
    rt.font_type = 8;
    rt.horizontal_align = 0;
    th19.render_text(text_renderer, &rt);

    rt.color = 0xffffffff;
    rt.font_type = 6;
    th19.render_text(text_renderer, &rt);
}

pub struct SharedRoom {
    menu_id: u8,
    menu: CommonMenu,
}

impl SharedRoom {
    pub fn new() -> Self {
        let (menu_id, menu) = make_enter_menu();
        Self { menu_id, menu }
    }

    pub fn on_input_menu(
        &mut self,
        current_input: InputValue,
        prev_input: InputValue,
        th19: &Th19,
        private_match_opponent: &mut Option<SharedRoomOpponent>,
    ) -> Option<LobbyScene> {
        if let Some(private_match_opponent) = private_match_opponent {
            private_match_opponent.recv();
        }
        match self.menu.on_input_menu(current_input, prev_input, th19) {
            OnMenuInputResult::None => {
                if private_match_opponent.is_none() {
                    if self.menu_id != 0 {
                        (self.menu_id, self.menu) = make_enter_menu();
                    }
                } else {
                    //
                    if self.menu_id != 1 {
                        (self.menu_id, self.menu) = make_leave_menu();
                    }
                }
                None
            }
            OnMenuInputResult::Cancel => Some(LobbyScene::Root),
            OnMenuInputResult::Action(MenuAction::SubScene(_)) => unreachable!(),
            OnMenuInputResult::Action(MenuAction::Action(action, _)) => match action {
                0 => {
                    *private_match_opponent = Some(SharedRoomOpponent::new(
                        th19.online_vs_mode().room_name().to_owned(),
                    ));
                    (self.menu_id, self.menu) = make_leave_menu();
                    None
                }
                1 => {
                    *private_match_opponent = None;
                    (self.menu_id, self.menu) = make_enter_menu();
                    None
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn on_render_texts(
        &self,
        private_match_opponent: Option<&SharedRoomOpponent>,
        th19: &Th19,
        text_renderer: *const c_void,
    ) {
        self.menu.on_render_texts(th19, text_renderer);

        if private_match_opponent.is_none() {
            let room_name = th19.online_vs_mode().room_name();
            render_room_name(th19, text_renderer, room_name);
        }

        if let Some(private_match_opponent) = private_match_opponent {
            let elapsed = private_match_opponent.elapsed();
            let progress = if let Some(error) = private_match_opponent.error() {
                let error_msg = format!("Failed: {}", error);
                render_text_line(th19, text_renderer, 13, error_msg.as_bytes());
                0.0
            } else {
                elapsed.as_secs_f64() / 4.0
            };
            let text = progress_text(progress);
            render_progress(th19, text_renderer, &text);
        }
    }
}