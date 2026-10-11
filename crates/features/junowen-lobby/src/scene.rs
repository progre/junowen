use crate::menu;

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

pub type CommonMenu = menu::CommonMenu<LobbyScene>;
pub type Menu = menu::Menu<LobbyScene>;
pub type MenuItem = menu::MenuItem<LobbyScene>;
pub type OnMenuInputResult = menu::OnMenuInputResult<LobbyScene>;
