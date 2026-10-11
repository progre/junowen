use tracing::info;

use super::{SpectatorHostDelegation, SpectatorSessionMessage};

/// 観戦ホストとして配下の観戦者に中継するメッセージの記録
///
/// 観戦ホストは、自分が処理したのと同じ順序で配下の観戦者に試合の進行を送る
#[derive(Default)]
pub struct RelayOutbox {
    delegation: Option<SpectatorHostDelegation>,
    /// 観戦ホストに任命されているか
    active: bool,
    /// 処理済みのメッセージのうち、まだ中継していないもの
    messages: Vec<SpectatorSessionMessage>,
}

impl RelayOutbox {
    /// 観戦ホストへの任命を取り込み、それ以外のメッセージを返す
    pub fn filter_delegation(
        &mut self,
        msg: SpectatorSessionMessage,
    ) -> Option<SpectatorSessionMessage> {
        match msg {
            SpectatorSessionMessage::DelegateSpectatorHost(delegation) => {
                info!("delegated spectator host");
                self.delegation = Some(delegation);
                self.active = true;
                None
            }
            msg => Some(msg),
        }
    }

    /// 観戦ホストに任命された場合、その任命を返す
    pub fn take_delegation(&mut self) -> Option<SpectatorHostDelegation> {
        self.delegation.take()
    }

    pub fn push(&mut self, msg: SpectatorSessionMessage) {
        if self.active {
            self.messages.push(msg);
        }
    }

    /// 試合中に処理したメッセージを記録する
    ///
    /// `InitGame` はバッファに戻されることがあるため、試合の開始時に記録する
    pub fn push_processed(&mut self, msg: &SpectatorSessionMessage) {
        if matches!(
            msg,
            SpectatorSessionMessage::InitRound(_) | SpectatorSessionMessage::Inputs(..)
        ) {
            self.push(msg.clone());
        }
    }

    pub fn take_messages(&mut self) -> Vec<SpectatorSessionMessage> {
        std::mem::take(&mut self.messages)
    }
}
