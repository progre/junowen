use junowen_lib::{Th19, structs::app::MainMenu};

use crate::{
    helper::pushed_f1,
    session::spectator::{SpectatorSession as SpectatorSessionProps, SpectatorSessionMessage},
    signaling::waiting_for_match::WaitingForSpectator,
    state::spectator_host::SpectatorHostState,
};

/// 観戦ホストに任命された場合に、配下の観戦者の受け付けと試合の進行の中継を行う
#[derive(Default)]
pub struct Relay(Option<SpectatorHostState>);

impl Relay {
    pub fn state(&self) -> Option<&SpectatorHostState> {
        self.0.as_ref()
    }

    pub fn start_if_delegated(&mut self, props: &mut SpectatorSessionProps) {
        if let Some(delegation) = props.relay_outbox_mut().take_delegation() {
            let waiting = WaitingForSpectator::new(delegation.into_room());
            self.0 = Some(SpectatorHostState::new_relay(waiting));
        }
    }

    /// 配下の観戦者の接続を受け付ける
    pub fn accept_spectators(
        &mut self,
        props: &SpectatorSessionProps,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
    ) {
        let (Some(state), Some(initial)) = (&mut self.0, props.spectator_initial()) else {
            return;
        };
        state.update(pushed_f1(th19.input_devices()), main_menu, th19, initial);
    }

    /// 処理したメッセージを、処理した順に配下の観戦者へ送る
    pub fn forward(&mut self, props: &mut SpectatorSessionProps) {
        let messages = props.relay_outbox_mut().take_messages();
        let Some(state) = &mut self.0 else {
            return;
        };
        for msg in messages {
            match msg {
                SpectatorSessionMessage::InitGame(init) => state.start_game(init),
                SpectatorSessionMessage::InitRound(init) => state.send_init_round(init),
                SpectatorSessionMessage::Inputs(p1, p2) => state.send_inputs(p1, p2),
                SpectatorSessionMessage::InitSpectator(_)
                | SpectatorSessionMessage::DelegateSpectatorHost(_) => {}
            }
        }
    }

    pub fn end_game(&mut self) {
        if let Some(state) = &mut self.0 {
            state.end_game();
        }
    }
}
