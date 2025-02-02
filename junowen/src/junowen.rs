use std::ffi::c_void;

use junowen_lib::{
    Th19, Th19EventListener,
    structs::{app::ScreenId, others::RenderingText},
};
use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;

use crate::{
    custom_direct_3d::Direct3DDeviceEventListener,
    file::{Features, SettingsRepo},
    lobby::{self, Lobby, TitleMenuModifier},
    state::State,
};

pub struct Junowen {
    features: Vec<Features>,
    th19: &'static mut Th19,
    title_menu_modifier: TitleMenuModifier,
    lobby: Lobby,
    state: State,
    old_p1_idx: u32,
}

impl Junowen {
    pub async fn new(settings_repo: SettingsRepo, th19: &'static mut Th19) -> Self {
        Self {
            features: settings_repo.features().await,
            th19,
            title_menu_modifier: TitleMenuModifier::new(),
            lobby: Lobby::new(settings_repo),
            state: State::Standby,
            old_p1_idx: 0,
        }
    }
}

impl Th19EventListener for Junowen {
    fn on_input_players(&mut self) {
        let (changed, menu_opt) = self.state.update_state(self.th19, &mut self.lobby);
        self.state
            .update_th19_on_input_players(changed, menu_opt, self.th19);
    }

    fn on_input_menu(&mut self) {
        self.state
            .on_input_menu(self.th19, &mut self.title_menu_modifier, &mut self.lobby);
    }

    fn on_before_render_object(&self, obj: &c_void) -> bool {
        self.state
            .on_before_render_object(&self.title_menu_modifier, obj)
    }

    fn on_before_render_text(&self, text_renderer: &c_void, text: &mut RenderingText) {
        self.state
            .on_before_render_text(self.th19, &self.title_menu_modifier, text_renderer, text);
    }

    fn on_render_texts(&self, text_renderer: &c_void) {
        self.state.on_render_texts(
            &self.features,
            self.th19,
            &self.title_menu_modifier,
            &self.lobby,
            text_renderer,
        );
    }

    fn on_round_over(&mut self) {
        self.state.on_round_over(self.th19);
    }

    fn on_before_is_online_vs(&self) -> Option<u8> {
        self.state.on_before_is_online_vs()
    }

    fn on_before_rewrite_controller_assignments(&mut self) {
        self.old_p1_idx = self.th19.input_devices().p1_idx();
    }

    fn on_rewrite_controller_assignments(&mut self) {
        self.state
            .on_rewrite_controller_assignments(self.th19, self.old_p1_idx);
    }

    fn on_before_loaded_game_settings(&mut self) {
        self.state.on_before_loaded_game_settings(self.th19);
    }
}

impl Direct3DDeviceEventListener for &Junowen {
    fn on_before_present(&self, device: &IDirect3DDevice9) {
        if !self.title_menu_modifier.selected_junowen() {
            return;
        }
        let Some(main_menu) = self.th19.app().main_loop_tasks().find_main_menu() else {
            return;
        };
        if main_menu.screen_id() != ScreenId::PlayerMatchupSelect {
            return;
        }

        lobby::overlay(device, &self.lobby, self.th19.window_inner());
    }
}
