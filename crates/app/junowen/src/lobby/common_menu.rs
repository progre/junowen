use std::ffi::c_void;

use junowen_lib::{
    Th19,
    structs::{
        input_devices::{InputFlags, InputValue},
        others::RenderingText,
    },
};
use junowen_menu::{MenuDevice, MenuRenderer, MenuSound, MenuText};

pub use junowen_menu::{MenuButtons, menu_item_color, render_label_value};

#[derive(Clone, Copy, Debug)]
pub enum LobbyScene {
    Root,
    SharedRoom,
    ReservedRoom,
    TcpSignaling,
    PureP2pHost,
    PureP2pGuest,
    PureP2pSpectator,
}

pub type CommonMenu = junowen_menu::CommonMenu<LobbyScene>;
pub type Menu = junowen_menu::Menu<LobbyScene>;
pub type MenuItem = junowen_menu::MenuItem<LobbyScene>;
pub type OnMenuInputResult = junowen_menu::OnMenuInputResult<LobbyScene>;

pub fn to_menu_buttons(input: InputValue) -> MenuButtons {
    let bits = |flags: &[InputFlags]| {
        flags
            .iter()
            .enumerate()
            .filter(|&(_, &flag)| input.0.contains(flag))
            .fold(0u8, |acc, (i, _)| acc | (1 << i))
    };
    MenuButtons {
        up: input.0.contains(InputFlags::UP),
        down: input.0.contains(InputFlags::DOWN),
        decide: bits(&[InputFlags::SHOT, InputFlags::ENTER]),
        cancel: bits(&[InputFlags::CHARGE, InputFlags::BOMB, InputFlags::PAUSE]),
    }
}

pub struct Th19MenuDevice<'a>(pub &'a Th19);

impl MenuDevice for Th19MenuDevice<'_> {
    fn play_sound(&self, sound: MenuSound) {
        let id = match sound {
            MenuSound::Cancel => 0x09,
            MenuSound::Decide => 0x07,
            MenuSound::Select => 0x0a,
        };
        self.0.play_sound(self.0.sound_manager(), id, 0);
    }

    fn keyboard_raw_keys(&self) -> &[u8; 256] {
        self.0.input_devices().keyboard_input().raw_keys()
    }
}

pub struct Th19MenuRenderer<'a> {
    pub th19: &'a Th19,
    pub text_renderer: &'a c_void,
}

impl MenuRenderer for Th19MenuRenderer<'_> {
    fn render_text(&self, text: &MenuText) {
        let mut rt = RenderingText::default();
        rt.set_text(text.text);
        rt.set_x(text.x, self.th19.window_inner());
        rt.set_y(text.y, self.th19.window_inner());
        rt.color = text.color;
        rt.font_type = text.font_type;
        rt.drop_shadow = text.drop_shadow;
        rt.horizontal_align = text.horizontal_align;
        rt.vertical_align = text.vertical_align;
        self.th19.render_text(self.text_renderer, &rt);
    }
}
