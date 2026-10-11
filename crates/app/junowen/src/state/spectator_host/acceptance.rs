use junowen_lib::{Th19, structs::app::MainMenu};
use tracing::info;

use crate::{
    session::{spectator::SpectatorRelayRoom, spectator_host::SpectatorHostSession},
    signaling::waiting_for_match::WaitingForSpectator,
};

/// 観戦者の受け付け状態
pub enum Acceptance {
    /// 対戦者: 最初の観戦者を待ち受け、観戦ホストに任命する
    WaitingForSpectatorHost(WaitingForSpectator),
    /// 対戦者: 観戦ホストに受け付けを委譲している
    Delegated,
    /// 観戦ホスト: 観戦者を待ち受け、受け取った試合の進行を中継する
    Relaying(WaitingForSpectator),
}

/// 専有ルームの待ち受けは一度しか受信できないため、受信後に作り直す
fn rearm(waiting: &mut WaitingForSpectator, room: Option<SpectatorRelayRoom>) {
    if room.is_some() {
        *waiting = WaitingForSpectator::new(room);
    }
}

impl Acceptance {
    pub fn waiting(&self) -> Option<&WaitingForSpectator> {
        match self {
            Self::WaitingForSpectatorHost(waiting) | Self::Relaying(waiting) => Some(waiting),
            Self::Delegated => None,
        }
    }

    /// 観戦者の接続を受け付ける
    ///
    /// 対戦者が最初に受け付けた観戦者は観戦ホストに任命され、以降の受け付けは観戦ホストに委譲される。
    /// 観戦ホストが離脱して配下の観戦者もいなくなった場合は、Pure P2P で受け付けを再開する
    pub fn try_recv_session(
        &mut self,
        has_spectators: bool,
        pushed: bool,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
    ) -> Option<SpectatorHostSession> {
        let (waiting, delegates) = match &mut *self {
            Self::WaitingForSpectatorHost(waiting) => (waiting, true),
            Self::Relaying(waiting) => (waiting, false),
            Self::Delegated => {
                if !has_spectators {
                    // 専有ルームは観戦ホストの離脱時に削除されるため、Pure P2P で待ち受ける
                    info!("spectator host left. resume waiting for spectator");
                    *self = Self::WaitingForSpectatorHost(WaitingForSpectator::new(None));
                }
                return None;
            }
        };
        let session = waiting.try_recv_session(pushed, main_menu, th19)?;
        let room = waiting.room().cloned();
        if !delegates {
            rearm(waiting, room);
            return Some(session);
        }
        if let Err(err) = session.send_delegate_spectator_host(room.clone()) {
            info!("delegate spectator host failed: {:?}", err);
            rearm(waiting, room);
            return None;
        }
        // 専有ルームの待ち受けは観戦ホストが引き継ぐため、ここでは作り直さない
        // (作り直した待ち受けを破棄すると部屋が削除されてしまう)
        info!("delegated spectator host");
        *self = Self::Delegated;
        Some(session)
    }
}
