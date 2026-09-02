use crate::lobby::common_menu::{CommonMenu, Menu, MenuItem};

const HOST_LABEL: &str = "Connect as a Host";
const GUEST_LABEL: &str = "Connect as a Guest";
const LEAVE_LABEL: &str = "Leave";
const OFFLINE_LABEL: &str = "Offline Mode: ON";
const ONLINE_LABEL: &str = "Offline Mode: OFF";

pub const HOST_ACTION_ID: u8 = 0;
pub const GUEST_ACTION_ID: u8 = 3;
pub const CHANGE_ADDRESS_ACTION_ID: u8 = 11;
pub const ADDRESS_INPUT_ACTION_ID: u8 = 12;
pub const OFFLINE_ACTION_ID: u8 = 20;

pub const OFFLINE_ITEM_INDEX: usize = 3;

enum Role {
    Host,
    Guest,
}

fn make_menu() -> CommonMenu {
    let menu = Menu::new(
        "TCP Signaling",
        None,
        vec![
            MenuItem::plain(HOST_LABEL, HOST_ACTION_ID, true),
            MenuItem::plain(GUEST_LABEL, GUEST_ACTION_ID, true),
            MenuItem::text_input(
                "Change Address",
                CHANGE_ADDRESS_ACTION_ID,
                ADDRESS_INPUT_ACTION_ID,
                "Address",
            ),
            MenuItem::plain(OFFLINE_LABEL, OFFLINE_ACTION_ID, true),
        ],
        0,
    );
    CommonMenu::new(false, 240 + 56, menu)
}

/// 「TCP Signaling」メニューの見た目(ラベル・有効/無効)と役割(Host/Guest/待ち受けなし)の
/// 状態遷移をまとめる
pub struct TcpSignalingMenu {
    menu: CommonMenu,
    role: Option<Role>,
    offline: Option<bool>,
}

impl TcpSignalingMenu {
    pub fn new() -> Self {
        Self {
            menu: make_menu(),
            role: None,
            offline: None,
        }
    }

    pub fn common_menu(&self) -> &CommonMenu {
        &self.menu
    }

    pub fn common_menu_mut(&mut self) -> &mut CommonMenu {
        &mut self.menu
    }

    pub fn has_role(&self) -> bool {
        self.role.is_some()
    }

    pub fn offline(&self) -> Option<bool> {
        self.offline
    }

    /// STUN の到達性はネットワークによって変わるため、待ち時間を避けるかどうかを
    /// 手動で切り替えられるようにする
    pub fn set_offline(&mut self, offline: bool) {
        self.offline = Some(offline);
        let label = if offline { OFFLINE_LABEL } else { ONLINE_LABEL };
        self.menu.menu_mut().items_mut()[OFFLINE_ITEM_INDEX].set_label(label);
    }

    /// 他の接続方式(Shared Room)と同様に、接続待ち中も他の機能を使えるように
    /// サブメニューへは潜らずフラットなメニュー構成にし、キャンセルでは待機を破棄しない
    pub fn change_to_idle(&mut self) {
        self.role = None;
        let items = self.menu.menu_mut().items_mut();
        items[0].set_label(HOST_LABEL);
        items[0].set_enabled(true);
        items[1].set_label(GUEST_LABEL);
        items[1].set_enabled(true);
        items[2].set_enabled(true);
        items[3].set_enabled(true);
    }

    pub fn change_to_host(&mut self) {
        self.role = Some(Role::Host);
        let items = self.menu.menu_mut().items_mut();
        items[0].set_label(LEAVE_LABEL);
        items[1].set_enabled(false);
        items[2].set_enabled(false);
        items[3].set_enabled(false);
    }

    pub fn change_to_guest(&mut self) {
        self.role = Some(Role::Guest);
        let items = self.menu.menu_mut().items_mut();
        items[1].set_label(LEAVE_LABEL);
        items[0].set_enabled(false);
        items[2].set_enabled(false);
        items[3].set_enabled(false);
    }
}
