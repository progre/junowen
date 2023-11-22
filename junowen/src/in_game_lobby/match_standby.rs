use std::time::{Duration, Instant};

use anyhow::Error;
use derive_new::new;
use getset::Getters;
use junowen_lib::connection::signaling::socket::SignalingSocket;
use tokio::{
    sync::{
        mpsc::{self, error::TryRecvError},
        oneshot, watch,
    },
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, debug_span, info, Instrument};

use crate::{
    session::{battle::BattleSession, spectator::SpectatorSessionGuest},
    TOKIO_RUNTIME,
};

use super::signaling_server_conn::SignalingServerSocket;

#[derive(Getters)]
pub struct WaitingInSharedRoom {
    handle: JoinHandle<()>,
    room_name: String,
    created_at: Instant,
    errors: Vec<Error>,
    error_rx: mpsc::Receiver<Error>,
    session_rx: oneshot::Receiver<BattleSession>,
    abort_tx: watch::Sender<bool>,
}

impl WaitingInSharedRoom {
    pub fn new(room_name: String) -> Self {
        let (error_tx, error_rx) = mpsc::channel(1);
        let (session_tx, session_rx) = oneshot::channel();
        let (abort_tx, abort_rx) = watch::channel(false);

        let handle = {
            let mut room_name = room_name.clone();
            TOKIO_RUNTIME.spawn(async move {
                let mut origin = if cfg!(debug_assertions) {
                    "https://qayvs4nki2nl72kf4tn5h5yati0maxpe.lambda-url.ap-northeast-1.on.aws"
                        .into()
                } else {
                    "https://wxvo3rgklveqwyig4b3q5qupbq0mgvik.lambda-url.ap-northeast-1.on.aws"
                        .into()
                };
                let (conn, dc, host) = loop {
                    let mut socket =
                        SignalingServerSocket::new(origin, room_name.clone(), abort_rx.clone());
                    match socket.receive_signaling().await {
                        Ok(ok) => break ok,
                        Err(err) => {
                            if *abort_rx.borrow() {
                                info!("canceled");
                                return;
                            }
                            info!("Signaling failed: {}", err);
                            let _ = error_tx.send(err).await;
                            sleep(Duration::from_secs(3)).await;
                            (origin, room_name) = socket.into_inner();
                        }
                    }
                };
                info!("Signaling succeeded");
                let session = BattleSession::new(conn, dc, host);
                session_tx.send(session).map_err(|_| ()).unwrap();
            })
        };

        Self {
            handle,
            room_name,
            created_at: Instant::now(),
            errors: vec![],
            error_rx,
            session_rx,
            abort_tx,
        }
    }

    pub fn room_name(&self) -> &str {
        &self.room_name
    }

    pub fn elapsed(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn recv(&mut self) {
        if let Ok(error) = self.error_rx.try_recv() {
            self.errors.push(error);
        }
    }

    pub fn errors(&self) -> &[Error] {
        &self.errors
    }

    pub fn try_into_session(mut self) -> Result<BattleSession, Self> {
        self.session_rx.try_recv().map_err(|_| self)
    }
}

impl Drop for WaitingInSharedRoom {
    fn drop(&mut self) {
        let _ = self.abort_tx.send(true);
        if self.handle.is_finished() {
            return;
        }
        let span = debug_span!("drop()", "{:?}", std::thread::current().id());
        let abort_handle = self.handle.abort_handle();
        TOKIO_RUNTIME.spawn(
            async move {
                debug!("drop sleep");
                sleep(Duration::from_secs(10)).await;
                abort_handle.abort();
                debug!("abort_handle.abort()");
            }
            .instrument(span),
        );
    }
}

#[derive(new)]
pub struct WaitingForPureP2pOpponent {
    battle_session_rx: mpsc::Receiver<BattleSession>,
}

pub enum WaitingForOpponent {
    SharedRoom(WaitingInSharedRoom),
    PureP2p(WaitingForPureP2pOpponent),
}

impl WaitingForOpponent {
    pub fn try_into_session(self) -> Result<BattleSession, Self> {
        match self {
            Self::SharedRoom(room) => room
                .try_into_session()
                .map_err(WaitingForOpponent::SharedRoom),
            Self::PureP2p(mut pure_p2p) => pure_p2p
                .battle_session_rx
                .try_recv()
                .map_err(|_| WaitingForOpponent::PureP2p(pure_p2p)),
        }
    }
}

#[derive(new)]
pub struct WaitingForPureP2pSpectator {
    spectator_session_guest_rx: mpsc::Receiver<SpectatorSessionGuest>,
}

pub enum WaitingForSpectator {
    PureP2p(WaitingForPureP2pSpectator),
}

impl WaitingForSpectator {
    pub fn try_recv_session(&mut self) -> Result<SpectatorSessionGuest, TryRecvError> {
        match self {
            Self::PureP2p(pure_p2p) => pure_p2p.spectator_session_guest_rx.try_recv(),
        }
    }
}

pub enum MatchStandby {
    Opponent(WaitingForOpponent),
    Spectator(WaitingForSpectator),
}

impl From<WaitingForPureP2pOpponent> for MatchStandby {
    fn from(value: WaitingForPureP2pOpponent) -> Self {
        MatchStandby::Opponent(WaitingForOpponent::PureP2p(value))
    }
}

impl From<WaitingForPureP2pSpectator> for MatchStandby {
    fn from(value: WaitingForPureP2pSpectator) -> Self {
        MatchStandby::Spectator(WaitingForSpectator::PureP2p(value))
    }
}
