use anyhow::{bail, Result};
use getset::Getters;
use junowen_lib::{MainMenu, ScreenId, Selection, Th19};
use tracing::info;

use crate::{
    session::{
        battle::BattleSession,
        spectator::{self, InitialState, SpectatorInitial},
        spectator_host::SpectatorHostSession,
        RoundInitial,
    },
    signaling::waiting_for_match::WaitingForSpectator,
};

fn create_spectator_initial(
    current_screen: ScreenId,
    selection: &Selection,
    battle_session: &BattleSession,
    local_player_name: String,
) -> SpectatorInitial {
    let p1_name = if battle_session.host() {
        local_player_name.to_owned()
    } else {
        battle_session.remote_player_name().clone()
    };
    let p2_name = if battle_session.host() {
        battle_session.remote_player_name().clone()
    } else {
        local_player_name.to_owned()
    };
    SpectatorInitial::new(
        p1_name,
        p2_name,
        battle_session
            .match_initial()
            .as_ref()
            .unwrap()
            .game_settings
            .clone(),
        InitialState::new(
            match current_screen {
                ScreenId::DifficultySelect => spectator::Screen::DifficultySelect,
                ScreenId::CharacterSelect => spectator::Screen::CharacterSelect,
                _ => unreachable!(),
            },
            selection.difficulty as u8,
            selection.p1().character as u8,
            selection.p1().card as u8,
            selection.p2().character as u8,
            selection.p2().card as u8,
        ),
    )
}

#[derive(Getters)]
pub struct SpectatorHostState {
    #[get = "pub"]
    waiting: WaitingForSpectator,
    sessions: Vec<SpectatorHostSession>,
}

impl SpectatorHostState {
    pub fn new(waiting: WaitingForSpectator) -> Self {
        Self {
            waiting,
            sessions: Vec::new(),
        }
    }

    pub fn count_spectators(&self) -> usize {
        self.sessions.len()
    }

    pub fn send_init_round_if_connected(&mut self, th19: &Th19) {
        self.sessions.retain(|session| {
            if let Err(err) = session.send_init_round(RoundInitial {
                seed1: th19.rand_seed1().unwrap(),
                seed2: th19.rand_seed2().unwrap(),
                seed3: th19.rand_seed3().unwrap(),
                seed4: th19.rand_seed4().unwrap(),
            }) {
                info!("spectator host error: {:?}", err);
                false
            } else {
                true
            }
        });
    }

    fn init_session(
        &self,
        session: &SpectatorHostSession,
        battle_session: &BattleSession,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
    ) -> Result<()> {
        let Some(main_menu) = main_menu else {
            bail!("spectator not supported yet.");
        };
        let vs_mode = th19.vs_mode();
        if main_menu.screen_id() != ScreenId::DifficultySelect
            || vs_mode.p1_card() != 0
            || vs_mode.p2_card() != 0
        {
            bail!(
                "spectator not supported yet. screen={:?}, p1_card={}, p2_card={}",
                main_menu.screen_id(),
                vs_mode.p1_card(),
                vs_mode.p2_card()
            );
        }
        session.send_init_spectator(create_spectator_initial(
            main_menu.screen_id(),
            th19.selection(),
            battle_session,
            th19.vs_mode().player_name().to_string(),
        ))?;
        session.send_init_round(RoundInitial {
            seed1: th19.rand_seed1().unwrap(),
            seed2: th19.rand_seed2().unwrap(),
            seed3: th19.rand_seed3().unwrap(),
            seed4: th19.rand_seed4().unwrap(),
        })?;
        Ok(())
    }

    pub fn update(
        &mut self,
        pushed: bool,
        main_menu: Option<&MainMenu>,
        th19: &Th19,
        battle_session: &BattleSession,
        p1_input: u16,
        p2_input: u16,
    ) {
        if let Some(session) = self.waiting.try_recv_session(pushed, main_menu, th19) {
            if let Err(err) = self.init_session(&session, battle_session, main_menu, th19) {
                info!("initialize spectator failed: {:?}", err);
            } else {
                self.sessions.push(session);
            }
        }
        self.sessions.retain(|session| {
            if let Err(err) = session.send_inputs(p1_input, p2_input) {
                info!("spectator host error: {:?}", err);
                return false;
            }
            true
        });
    }
}
