use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

use anyhow::Error;
use getset::Getters;
use junowen_lib::connection::{signaling::socket::SignalingSocket, DataChannel, PeerConnection};
use tokio::{
    sync::{
        mpsc::{self},
        oneshot, watch,
    },
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, debug_span, info, Instrument};

use crate::{
    in_game_lobby::waiting_for_match::{
        reserved_room_opponent_socket::SignalingServerReservedRoomOpponentSocket,
        shared_room_opponent_socket::SignalingServerSharedRoomOpponentSocket,
    },
    session::{battle::BattleSession, spectator::SpectatorSessionGuest},
    TOKIO_RUNTIME,
};

use super::reserved_room_spectator_socket::SignalingServerReservedRoomSpectatorSocket;

#[derive(Getters)]
pub struct WaitingInRoom<TSocket, TSession> {
    handle: JoinHandle<()>,
    room_name: String,
    created_at: Instant,
    errors: Vec<Error>,
    error_rx: mpsc::Receiver<Error>,
    session_rx: oneshot::Receiver<TSession>,
    abort_tx: watch::Sender<bool>,
    _phantom: PhantomData<TSocket>,
}

pub type WaitingForOpponentInSharedRoom =
    WaitingInRoom<SignalingServerSharedRoomOpponentSocket, BattleSession>;
pub type WaitingForOpponentInReservedRoom =
    WaitingInRoom<SignalingServerReservedRoomOpponentSocket, BattleSession>;
pub type WaitingForSpectatorHostInReservedRoom =
    WaitingInRoom<SignalingServerReservedRoomSpectatorSocket, SpectatorSessionGuest>;

impl<TSocket, TSession> WaitingInRoom<TSocket, TSession>
where
    TSocket: SignalingSocket + Send + 'static,
    TSession: Send + 'static,
{
    fn internal_new(
        create_socket: fn(
            origin: String,
            room_name: String,
            abort_rx: watch::Receiver<bool>,
        ) -> TSocket,
        create_session: fn(conn: PeerConnection, data_channel: DataChannel, host: bool) -> TSession,
        room_name: String,
    ) -> Self {
        let (error_tx, error_rx) = mpsc::channel(1);
        let (session_tx, session_rx) = oneshot::channel();
        let (abort_tx, abort_rx) = watch::channel(false);

        let handle = {
            let room_name = room_name.clone();
            TOKIO_RUNTIME.spawn(async move {
                let origin = if cfg!(debug_assertions) {
                    "https://qayvs4nki2nl72kf4tn5h5yati0maxpe.lambda-url.ap-northeast-1.on.aws"
                        .into()
                } else {
                    "https://wxvo3rgklveqwyig4b3q5qupbq0mgvik.lambda-url.ap-northeast-1.on.aws"
                        .into()
                };
                let mut socket = create_socket(origin, room_name, abort_rx.clone());
                let (conn, dc, host) = loop {
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
                        }
                    }
                };
                info!("Signaling succeeded");
                let session = create_session(conn, dc, host);
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
            _phantom: PhantomData,
        }
    }
}

impl WaitingForOpponentInSharedRoom {
    pub fn new(room_name: String) -> Self {
        Self::internal_new(
            SignalingServerSharedRoomOpponentSocket::new,
            BattleSession::new,
            room_name,
        )
    }
}

impl WaitingForOpponentInReservedRoom {
    pub fn new(room_name: String) -> Self {
        Self::internal_new(
            SignalingServerReservedRoomOpponentSocket::new,
            BattleSession::new,
            room_name,
        )
    }
}

impl WaitingForSpectatorHostInReservedRoom {
    pub fn new(room_name: String) -> Self {
        Self::internal_new(
            |origin, room_name, abort_rx| {
                SignalingServerReservedRoomSpectatorSocket::new(origin, room_name, None, abort_rx)
            },
            |pc, dc, host| {
                assert!(!host);
                SpectatorSessionGuest::new(pc, dc)
            },
            room_name,
        )
    }
}

impl<TSocket, TSession> WaitingInRoom<TSocket, TSession> {
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

    pub fn try_into_session(mut self) -> Result<TSession, Self> {
        self.session_rx.try_recv().map_err(|_| self)
    }
}

impl<TSocket, TSession> Drop for WaitingInRoom<TSocket, TSession> {
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
