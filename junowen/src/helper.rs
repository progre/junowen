use junowen_lib::{
    Th19,
    structs::{app::MainMenu, input_devices::InputDevices},
};
use tracing::warn;

pub fn inputed_number(input_devices: &InputDevices) -> Option<u8> {
    let raw_keys = input_devices.keyboard_input().raw_keys();
    (0..=9).find(|i| raw_keys[(b'0' + i) as usize] & 0x80 != 0)
}

pub fn pushed_f1(input_devices: &InputDevices) -> bool {
    let raw_keys = input_devices.keyboard_input().raw_keys();
    raw_keys[0x70] & 0x80 != 0
}

pub fn pushed_escape(input_devices: &InputDevices) -> bool {
    let raw_keys = input_devices.keyboard_input().raw_keys();
    raw_keys[0x1b] & 0x80 != 0
}

fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{:02x}", x))
        .collect::<Vec<_>>()
        .join(" ")
}

/// TODO: カードの選択状態がどこに保持されているかを調べるための一時的な診断ログ。原因がわかったら削除する
pub fn dump_card_candidates(label: &str, main_menu: &MainMenu, th19: &Th19) {
    let selection = th19.selection();
    let vs_mode = th19.vs_mode();
    let menu = main_menu.menu();
    warn!(
        "[card-diagnostics] {} screen={:?} selection.card=({}, {}) vs_mode.card=({}, {})",
        label,
        main_menu.screen_id(),
        selection.p1().card,
        selection.p2().card,
        vs_mode.p1_card(),
        vs_mode.p2_card(),
    );
    warn!(
        "[card-diagnostics] {} vs_mode+2ea00h={}",
        label,
        to_hex(vs_mode.raw_bytes_around_cards())
    );
    warn!(
        "[card-diagnostics] {} selection.p1={}",
        label,
        to_hex(selection.p1().raw_bytes())
    );
    warn!(
        "[card-diagnostics] {} selection.p2={}",
        label,
        to_hex(selection.p2().raw_bytes())
    );
    warn!(
        "[card-diagnostics] {} menu.p1_cursor={}",
        label,
        to_hex(menu.p1_cursor().raw_bytes())
    );
    warn!(
        "[card-diagnostics] {} menu.p2_cursor={}",
        label,
        to_hex(menu.p2_cursor().raw_bytes())
    );
}
