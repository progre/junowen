use std::{cell::OnceCell, ffi::c_void};

use crate::{
    Fn0b7d40, Fn0d5ae0, Fn0d6e10, Fn009fa0, Fn10f720, Fn1049e0, Fn011560, FnOfHookAssembly, Th19,
    structs::{others::RenderingText, selection::Selection},
};

pub trait Th19EventListener {
    fn on_input_players(&mut self);
    fn on_input_menu(&mut self);
    fn on_before_render_object(&self, obj: *const c_void) -> bool;
    fn on_before_render_text(&self, text_renderer: &c_void, text: &mut RenderingText);
    fn on_render_texts(&self, text_renderer: &c_void);
    fn on_round_over(&mut self);
    fn on_before_is_online_vs(&self) -> Option<u8>;
    fn on_before_rewrite_controller_assignments(&mut self);
    fn on_rewrite_controller_assignments(&mut self);
    fn on_before_loaded_game_settings(&mut self);
}

extern "fastcall" fn on_input_players() {
    let dispatcher = th19_event_dispatcher_mut();
    dispatcher.listener.on_input_players();

    if let Some(func) = dispatcher.old_on_input_players {
        func()
    }
}

extern "fastcall" fn on_input_menu() {
    let dispatcher = th19_event_dispatcher_mut();
    dispatcher.listener.on_input_menu();

    if let Some(func) = dispatcher.old_on_input_menu {
        func()
    }
}

extern "thiscall" fn render_object(this: *const c_void, obj: *const c_void) {
    let dispatcher = th19_event_dispatcher();
    if !dispatcher.listener.on_before_render_object(obj) {
        return;
    }
    (dispatcher.old_fn_from_0bed70_00fc)(this, obj);
}

extern "thiscall" fn render_text(text_renderer: *const c_void, text: *mut RenderingText) -> u32 {
    let dispatcher = th19_event_dispatcher();
    let text = unsafe { text.as_mut().unwrap() };
    dispatcher
        .listener
        .on_before_render_text(unsafe { &*text_renderer }, text);
    (dispatcher.old_fn_from_0d6e10_0039)(text_renderer, text)
}

extern "thiscall" fn on_render_texts(text_renderer: *const c_void, arg: *const c_void) -> u32 {
    let dispatcher = th19_event_dispatcher();
    let ret = (dispatcher.old_fn_from_0d7180_0008)(text_renderer, arg);
    dispatcher
        .listener
        .on_render_texts(unsafe { &*text_renderer });
    ret
}

extern "fastcall" fn on_round_over() {
    let dispatcher = th19_event_dispatcher_mut();
    (dispatcher.old_fn_from_11f870_034c)();
    dispatcher.listener.on_round_over();
}

/// for pause menu online vs view
extern "thiscall" fn fn_from_1243f0_00f9(this: *const Selection) -> u8 {
    let dispatcher = th19_event_dispatcher();
    if let Some(result) = dispatcher.listener.on_before_is_online_vs() {
        return result;
    }
    (dispatcher.old_fn_from_1243f0_00f9)(this)
}

/// for pause menu online vs view
extern "thiscall" fn fn_from_1243f0_0320(this: *const Selection) -> u8 {
    let dispatcher = th19_event_dispatcher();
    if let Some(result) = dispatcher.listener.on_before_is_online_vs() {
        return result;
    }
    (dispatcher.old_fn_from_1243f0_0320)(this)
}

extern "fastcall" fn on_rewrite_controller_assignments() {
    let dispatcher = th19_event_dispatcher_mut();
    let listener = &mut dispatcher.listener;
    listener.on_before_rewrite_controller_assignments();
    (dispatcher.old_fn_from_13f9d0_0345)();
    listener.on_rewrite_controller_assignments();
}

extern "thiscall" fn on_loaded_game_settings(this: *const c_void, arg1: u32) -> u32 {
    let dispatcher = th19_event_dispatcher_mut();
    dispatcher.listener.on_before_loaded_game_settings();
    (dispatcher.old_fn_from_13f9d0_0446)(this, arg1)
}

pub struct Th19EventDispatcher {
    old_on_input_players: Option<FnOfHookAssembly>,
    old_on_input_menu: Option<FnOfHookAssembly>,
    old_fn_from_0bed70_00fc: Fn0b7d40,
    old_fn_from_0d6e10_0039: Fn0d5ae0,
    old_fn_from_0d7180_0008: Fn0d6e10,
    old_fn_from_11f870_034c: Fn1049e0,
    old_fn_from_1243f0_00f9: Fn011560,
    old_fn_from_1243f0_0320: Fn011560,
    old_fn_from_13f9d0_0345: Fn10f720,
    old_fn_from_13f9d0_0446: Fn009fa0,
    listener: &'static mut dyn Th19EventListener,
}

impl Th19EventDispatcher {
    pub fn init(th19: &mut Th19, listener: &'static mut dyn Th19EventListener) {
        let (old_on_input_players, apply_hook_on_input_players) =
            th19.hook_on_input_players(on_input_players);
        let (old_on_input_menu, apply_hook_on_input_menu) = th19.hook_on_input_menu(on_input_menu);
        let (old_fn_from_0bed70_00fc, apply_hook_0bed70_00fc) =
            th19.hook_0bed70_00fc(render_object);
        let (old_fn_from_0d6e10_0039, apply_hook_0d6e10_0039) = th19.hook_0d6e10_0039(render_text);
        let (old_fn_from_0d7180_0008, apply_hook_0d7180_0008) =
            th19.hook_0d7180_0008(on_render_texts);
        let (old_fn_from_11f870_034c, apply_hook_11f870_034c) =
            th19.hook_11f870_034c(on_round_over);
        let (old_fn_from_1243f0_00f9, apply_hook_1243f0_00f9) =
            th19.hook_1243f0_00f9(fn_from_1243f0_00f9);
        let (old_fn_from_1243f0_0320, apply_hook_1243f0_0320) =
            th19.hook_1243f0_0320(fn_from_1243f0_0320);
        let (old_fn_from_13f9d0_0345, apply_hook_13f9d0_0345) =
            th19.hook_13f9d0_0345(on_rewrite_controller_assignments);
        let (old_fn_from_13f9d0_0446, apply_hook_13f9d0_0446) =
            th19.hook_13f9d0_0446(on_loaded_game_settings);

        let dispatcher = Th19EventDispatcher {
            old_on_input_players,
            old_on_input_menu,
            old_fn_from_0bed70_00fc,
            old_fn_from_0d6e10_0039,
            old_fn_from_0d7180_0008,
            old_fn_from_11f870_034c,
            old_fn_from_1243f0_00f9,
            old_fn_from_1243f0_0320,
            old_fn_from_13f9d0_0345,
            old_fn_from_13f9d0_0446,
            listener,
        };
        let dispatcher_ptr = &raw const TH19_EVENT_DISPATCHER;
        unsafe { dispatcher_ptr.as_ref() }
            .unwrap()
            .set(dispatcher)
            .map_err(|_| ())
            .unwrap();

        apply_hook_on_input_players(th19);
        apply_hook_on_input_menu(th19);
        apply_hook_0bed70_00fc(th19);
        apply_hook_0d6e10_0039(th19);
        apply_hook_0d7180_0008(th19);
        apply_hook_11f870_034c(th19);
        apply_hook_1243f0_00f9(th19);
        apply_hook_1243f0_0320(th19);
        apply_hook_13f9d0_0345(th19);
        apply_hook_13f9d0_0446(th19);
    }
}

static mut TH19_EVENT_DISPATCHER: OnceCell<Th19EventDispatcher> = OnceCell::new();

fn th19_event_dispatcher() -> &'static Th19EventDispatcher {
    let dispatcher = unsafe { (&raw const TH19_EVENT_DISPATCHER).as_ref() }.unwrap();
    dispatcher.get().unwrap()
}
fn th19_event_dispatcher_mut() -> &'static mut Th19EventDispatcher {
    let dispatcher = unsafe { (&raw mut TH19_EVENT_DISPATCHER).as_mut() }.unwrap();
    dispatcher.get_mut().unwrap()
}
