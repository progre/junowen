use junowen_lib::{Th19, structs::selection::Player};

use crate::session::{
    RoundInitial,
    battle::BattleSession,
    spectator::{GameInitial, PlayerGameInitial, SpectatorInitial},
};

pub fn current_round_initial(th19: &Th19) -> RoundInitial {
    RoundInitial {
        seed1: th19.rand_seed1().unwrap(),
        seed2: th19.rand_seed2().unwrap(),
        seed3: th19.rand_seed3().unwrap(),
        seed4: th19.rand_seed4().unwrap(),
    }
}

pub fn current_game_initial(th19: &Th19) -> GameInitial {
    let selection = th19.selection();
    let player =
        |player: &Player| PlayerGameInitial::new(player.character as u8, player.card as u8);
    GameInitial::new(
        selection.difficulty as u8,
        player(selection.p1()),
        player(selection.p2()),
        current_round_initial(th19),
    )
}

/// 観戦者が接続した直後に、プレイヤー名と設定を観戦者へ渡す
pub trait SpectatorInitialSource {
    fn spectator_initial(&self, th19: &Th19) -> SpectatorInitial;
}

impl SpectatorInitialSource for BattleSession {
    fn spectator_initial(&self, th19: &Th19) -> SpectatorInitial {
        let local_player_name = th19.vs_mode().player_name().to_string();
        let remote_player_name = self.remote_player_name().clone();
        let (p1_name, p2_name) = if self.host() {
            (local_player_name, remote_player_name)
        } else {
            (remote_player_name, local_player_name)
        };
        SpectatorInitial::new(
            p1_name,
            p2_name,
            self.match_initial().as_ref().unwrap().game_settings.clone(),
        )
    }
}

/// 観戦ホストは、対戦者から受け取った情報をそのまま配下の観戦者に送る
impl SpectatorInitialSource for SpectatorInitial {
    fn spectator_initial(&self, _th19: &Th19) -> SpectatorInitial {
        self.clone()
    }
}
