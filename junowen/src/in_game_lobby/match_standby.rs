use std::time::{Duration, Instant};

use anyhow::Error;
use derive_new::new;
use getset::Getters;
use junowen_lib::connection::signaling::socket::SignalingSocket;
use tokio::{
    sync::{
        mpsc::{self, error::TryRecvError},
        oneshot,
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
pub struct SharedRoomOpponent {
    handle: JoinHandle<()>,
    room_name: String,
    created_at: Instant,
    error: Option<Error>,
    error_rx: oneshot::Receiver<Error>,
    session_rx: oneshot::Receiver<BattleSession>,
    abort_tx: Option<oneshot::Sender<()>>,
}

impl SharedRoomOpponent {
    pub fn new(room_name: String) -> Self {
        let (error_tx, error_rx) = oneshot::channel();
        let (session_tx, session_rx) = oneshot::channel();
        let (abort_tx, abort_rx) = oneshot::channel();

        let origin = if cfg!(debug_assertions) {
            "https://qayvs4nki2nl72kf4tn5h5yati0maxpe.lambda-url.ap-northeast-1.on.aws".into()
        } else {
            "https://wxvo3rgklveqwyig4b3q5qupbq0mgvik.lambda-url.ap-northeast-1.on.aws".into()
        };
        let handle = {
            let room_name = room_name.clone();
            TOKIO_RUNTIME.spawn(async move {
                let mut socket = SignalingServerSocket::new(origin, room_name, abort_rx);
                let (conn, dc, host) = match socket.receive_signaling().await {
                    Ok(ok) => ok,
                    Err(err) => {
                        info!("Signaling failed: {}", err);
                        let _ = error_tx.send(err);
                        return;
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
            error: None,
            error_rx,
            session_rx,
            abort_tx: Some(abort_tx),
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
            self.error = Some(error);
        }
    }

    pub fn error(&self) -> Option<&Error> {
        self.error.as_ref()
    }

    pub fn try_into_session(mut self) -> Result<BattleSession, Self> {
        self.session_rx.try_recv().map_err(|_| self)
    }
}

impl Drop for SharedRoomOpponent {
    fn drop(&mut self) {
        let _ = self.abort_tx.take().unwrap().send(());
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
pub struct PureP2pOpponent {
    battle_session_rx: mpsc::Receiver<BattleSession>,
}

pub enum Opponent {
    SharedRoom(SharedRoomOpponent),
    PureP2p(PureP2pOpponent),
}

impl Opponent {
    pub fn try_into_session(self) -> Result<BattleSession, Self> {
        match self {
            Self::SharedRoom(private_match) => private_match
                .try_into_session()
                .map_err(Opponent::SharedRoom),
            Self::PureP2p(mut pure_p2p) => pure_p2p
                .battle_session_rx
                .try_recv()
                .map_err(|_| Opponent::PureP2p(pure_p2p)),
        }
    }
}

#[derive(new)]
pub struct PureP2pSpectator {
    spectator_session_guest_rx: mpsc::Receiver<SpectatorSessionGuest>,
}

pub enum Spectator {
    PureP2p(PureP2pSpectator),
}

impl Spectator {
    pub fn try_recv_session(&mut self) -> Result<SpectatorSessionGuest, TryRecvError> {
        match self {
            Self::PureP2p(pure_p2p) => pure_p2p.spectator_session_guest_rx.try_recv(),
        }
    }
}

pub enum MatchStandby {
    Opponent(Opponent),
    Spectator(Spectator),
}

impl From<PureP2pOpponent> for MatchStandby {
    fn from(value: PureP2pOpponent) -> Self {
        MatchStandby::Opponent(Opponent::PureP2p(value))
    }
}

impl From<PureP2pSpectator> for MatchStandby {
    fn from(value: PureP2pSpectator) -> Self {
        MatchStandby::Spectator(Spectator::PureP2p(value))
    }
}
