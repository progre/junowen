use std::{f64::consts::PI, ffi::c_void};

use junowen_lib::{InputValue, RenderingText, Th19};

use super::{
    super::match_standby::WaitingInSharedRoom,
    common_menu::{CommonMenu, LobbyScene, MenuAction, MenuDefine, MenuItem, OnMenuInputResult},
    helper::render_text_line,
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

fn progress_alphas(progress: f64) -> Vec<u8> {
    const LENGTH: f64 = 20.0;
    let progress = progress / 2.0 % 1.0;

    // 4PI ごとに波と凪が交互に来る関数
    let curve = |x: f64| ((x + PI).cos() + 1.0) / 2.0 * ((x + PI) / 2.0).cos().ceil();

    (0..LENGTH as usize)
        .map(|i| {
            (curve((i as f64 / LENGTH / 2.0 - progress) * 4.0 * PI) * 0xff as f64).ceil() as u8
        })
        .collect()
}

/// アルファと cos カーブを使った表現
/// ボツ
#[allow(unused)]
fn render_progress_alpha(th19: &Th19, text_renderer: *const c_void, progress: f64) {
    let text = b"|                    |";
    let x = 640;
    let y = 160 + 32 * 11;
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.x = (x * th19.screen_width().unwrap() / 1280) as f32;
    rt.y = (y * th19.screen_height().unwrap() / 960) as f32;
    rt.color = 0xff000000;
    rt.font_type = 8;
    rt.horizontal_align = 0;
    th19.render_text(text_renderer, &rt);

    rt.color = 0xffffffff;
    rt.font_type = 6;
    th19.render_text(text_renderer, &rt);

    for (i, &alpha) in progress_alphas(progress).iter().enumerate() {
        let x = (650 - 200 + i * 20) as u32;

        rt.set_text(b"-");
        rt.x = (x * th19.screen_width().unwrap() / 1280) as f32;
        rt.color = (0xff - alpha) as u32 * 0x1000000;
        rt.font_type = 8;
        th19.render_text(text_renderer, &rt);
        rt.color |= 0x00ffffff;
        rt.font_type = 6;
        th19.render_text(text_renderer, &rt);

        rt.set_text(b"#");
        rt.x = (x * th19.screen_width().unwrap() / 1280) as f32;
        rt.color = alpha as u32 * 0x1000000;
        rt.font_type = 8;
        th19.render_text(text_renderer, &rt);
        rt.color |= 0x00ffffff;
        rt.font_type = 6;
        th19.render_text(text_renderer, &rt);
    }
}

fn progress_text(progress: f64) -> Vec<u8> {
    const BUFFER_TIME: f64 = 0.25;
    const LENGTH: f64 = 20.0 * (1.0 + BUFFER_TIME);
    let progress = ((progress / (1.0 + BUFFER_TIME) + 1.0) % 2.0 - 1.0) * LENGTH;
    let mut progress_text = vec![];
    let (progress, left_char, right_char, left_len) = if progress >= 0.0 {
        (progress, b'#', b'-', progress as usize)
    } else {
        let progress = -progress;
        (progress, b'-', b'#', LENGTH as usize - progress as usize)
    };
    let right_len = LENGTH as usize - left_len;
    progress_text.append(&mut vec![left_char; left_len]);
    if progress < LENGTH {
        progress_text.push(b'#');
    }
    progress_text.append(&mut vec![right_char; right_len]);

    let mut text = vec![b'['];
    progress_text[0..20].iter().for_each(|&x| text.push(x));
    text.push(b']');
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

fn render_progress_item(th19: &Th19, text_renderer: *const c_void, alpha: u8, text: &[u8]) {
    let x = (640 * th19.screen_width().unwrap() / 1280) as f32;
    let y = ((160 + 32 * 11) * th19.screen_height().unwrap() / 960) as f32;
    let mut rt = RenderingText::default();
    rt.set_text(text);
    rt.x = x;
    rt.y = y;
    rt.color = alpha as u32 * 0x1000000;
    rt.font_type = 8;
    rt.horizontal_align = 0;
    th19.render_text(text_renderer, &rt);

    rt.color = (alpha as u32 * 0x1000000) | 0x00ffffff;
    rt.font_type = 6;
    th19.render_text(text_renderer, &rt);
}

fn render_progress(th19: &Th19, text_renderer: *const c_void, progress: f64) {
    let base_text = progress_text(progress);
    render_progress_item(th19, text_renderer, 0xff, &base_text);
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
        waiting: &mut Option<WaitingInSharedRoom>,
    ) -> Option<LobbyScene> {
        if let Some(waiting) = waiting {
            waiting.recv();
        }
        match self.menu.on_input_menu(current_input, prev_input, th19) {
            OnMenuInputResult::None => {
                if waiting.is_none() {
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
                    *waiting = Some(WaitingInSharedRoom::new(
                        th19.online_vs_mode().room_name().to_owned(),
                    ));
                    (self.menu_id, self.menu) = make_leave_menu();
                    None
                }
                1 => {
                    *waiting = None;
                    (self.menu_id, self.menu) = make_enter_menu();
                    None
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn on_render_texts(
        &self,
        waiting: Option<&WaitingInSharedRoom>,
        th19: &Th19,
        text_renderer: *const c_void,
    ) {
        self.menu.on_render_texts(th19, text_renderer);

        if waiting.is_none() {
            let room_name = th19.online_vs_mode().room_name();
            render_room_name(th19, text_renderer, room_name);
        }

        if let Some(waiting) = waiting {
            let elapsed = waiting.elapsed();
            render_progress(th19, text_renderer, elapsed.as_secs_f64() / 4.0);
            for (i, error) in waiting.errors().iter().rev().enumerate() {
                let error_msg = format!("Failed: {}", error);
                render_text_line(th19, text_renderer, 13 + i as u32, error_msg.as_bytes());
            }
        }
    }
}