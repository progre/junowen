use std::ffi::c_void;

use getset::CopyGetters;
use junowen_lib::{
    structs::app::{MainMenu, ScreenId},
    structs::input_devices::{Input, InputFlags, InputValue},
    structs::others::RenderingText,
    Fn0d5ae0, Th19,
};

use super::helper::menu_item_color;

fn direction(input: &Input, flag: InputFlags) -> bool {
    input.current().0 & InputValue::from(flag).0 != None
        && (input.prev().0 & InputValue::from(flag).0 == None
            ||
            // At this timing, repeat is not completed, so judge by count
            match flag {
                InputFlags::UP => input.up_repeat_count(),
                InputFlags::DOWN => input.down_repeat_count(),
                InputFlags::LEFT => input.left_repeat_count(),
                InputFlags::RIGHT => input.right_repeat_count(),
                _ => unreachable!(),
            } == 25)
}

#[derive(CopyGetters)]
pub struct TitleMenuModifier {
    first_time: bool,
    #[get_copy = "pub"]
    selected_junowen: bool,
}

impl TitleMenuModifier {
    pub fn new() -> Self {
        Self {
            first_time: true,
            selected_junowen: false,
        }
    }

    pub fn start_lobby(&mut self, main_menu: &MainMenu) -> bool {
        if main_menu.screen_id() == ScreenId::PlayerMatchupSelect && self.selected_junowen {
            self.first_time = true;
            true
        } else {
            false
        }
    }

    pub fn on_input_menu(&mut self, main_menu: &mut MainMenu, th19: &mut Th19) {
        debug_assert_eq!(main_menu.screen_id(), ScreenId::Title);
        let menu = main_menu.menu_mut();
        if menu.num_disabled() > 0 {
            return;
        }
        if self.first_time {
            self.first_time = false;
            if self.selected_junowen {
                menu.set_cursor(2);
            }
        }
        match (menu.cursor(), self.selected_junowen) {
            (2, false) => {
                if direction(th19.menu_input(), InputFlags::DOWN) {
                    self.selected_junowen = true;
                    menu.set_cursor(1);
                }
            }
            (2, true) => {
                if th19.menu_input().decide() {
                    th19.menu_input_mut().set_current(InputFlags::SHOT.into());
                    menu.set_cursor(1);
                }
                if direction(th19.menu_input(), InputFlags::UP) {
                    self.selected_junowen = false;
                    menu.set_cursor(3);
                }
                if direction(th19.menu_input(), InputFlags::DOWN) {
                    self.selected_junowen = false;
                }
            }
            (3, _) => {
                if direction(th19.menu_input(), InputFlags::UP) {
                    self.selected_junowen = true;
                }
            }
            (7, _) => {
                self.selected_junowen = false;
            }
            _ => {}
        }
    }

    pub fn render_text(
        &self,
        main_menu: &MainMenu,
        th19: &Th19,
        old: Fn0d5ae0,
        text_renderer: *const c_void,
        rendering_text: &mut RenderingText,
    ) -> u32 {
        if main_menu.screen_id() != ScreenId::Title {
            return old(text_renderer, rendering_text);
        }
        let menu = main_menu.menu();
        if menu.num_disabled() > 0 {
            return old(text_renderer, rendering_text);
        }
        let text = rendering_text.text().unwrap().to_string_lossy().to_string();
        if ["Story Mode", "VS Mode", "Online VS Mode"].contains(&text.as_str()) {
            rendering_text.y -= (50 * th19.screen_height().unwrap() / 960) as f32;
        }
        let selected_junowen = [1, 2].contains(&menu.cursor()) && self.selected_junowen;
        if text == "VS Mode" && selected_junowen {
            // disable decide
            rendering_text.color = menu_item_color(rendering_text.font_type, true, false);
        }
        if text == "Online VS Mode" {
            if selected_junowen {
                // disable shake
                rendering_text.x = (64 * th19.screen_width().unwrap() / 1280) as f32;
                rendering_text.y = (550 * th19.screen_height().unwrap() / 960) as f32;
                // reset color
                rendering_text.color = menu_item_color(rendering_text.font_type, true, false);
            }
            {
                let mut rendering_text = RenderingText::default();
                rendering_text.set_text(b"Ju.N.Owen");
                rendering_text.x = (64 * th19.screen_width().unwrap() / 1280) as f32;
                rendering_text.y = (600 * th19.screen_height().unwrap() / 960) as f32;
                rendering_text.color = menu_item_color(9, true, selected_junowen);
                rendering_text.font_type = 9;
                th19.render_text(text_renderer, &rendering_text);

                rendering_text.color = if selected_junowen {
                    0xffff80e3
                } else {
                    0xff806079
                };
                rendering_text.font_type = 7;
                th19.render_text(text_renderer, &rendering_text);
            }
        }

        old(text_renderer, rendering_text)
    }
}
