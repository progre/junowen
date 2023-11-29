use std::time::{Duration, Instant};

use anyhow::Error;
use getset::Getters;
use junowen_lib::connection::{signaling::socket::SignalingSocket, DataChannel, PeerConnection};
use tokio::{
    sync::{
        mpsc::{self},
        oneshot::{self, error::TryRecvError},
        watch,
    },
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, debug_span, info, Instrument};

use crate::{
    session::{
        battle::BattleSession, spectator::SpectatorSession, spectator_host::SpectatorHostSession,
    },
    signaling::waiting_for_match::{
        reserved_room_opponent_socket::SignalingServerReservedRoomOpponentSocket,
        reserved_room_spectator_host_socket::SignalingServerReservedRoomSpectatorHostSocket,
        shared_room_opponent_socket::SignalingServerSharedRoomOpponentSocket,
        waiting_for_spectator::WaitingForPureP2pSpectator,
    },
    TOKIO_RUNTIME,
};

use super::{
    reserved_room_spectator_socket::SignalingServerReservedRoomSpectatorSocket, WaitingForSpectator,
};

pub struct RoomKey(String);

#[derive(Getters)]
pub struct WaitingInRoom<TSession> {
    handle: JoinHandle<()>,
    room_name: String,
    created_at: Instant,
    errors: Vec<Error>,
    error_rx: mpsc::Receiver<Error>,
    session_rx: oneshot::Receiver<TSession>,
    abort_tx: watch::Sender<bool>,
}

pub type WaitingForOpponentInSharedRoom = WaitingInRoom<BattleSession>;
pub type WaitingForOpponentInReservedRoom = WaitingInRoom<(BattleSession, Option<RoomKey>)>;
pub type WaitingForSpectatorInReservedRoom = WaitingInRoom<(SpectatorHostSession, RoomKey)>;
pub type WaitingForSpectatorHostInReservedRoom = WaitingInRoom<SpectatorSession>;

impl<TSession> WaitingInRoom<TSession>
where
    TSession: Send + 'static,
{
    fn internal_new<T>(
        create_socket: impl FnOnce(String, String, watch::Receiver<bool>) -> T + Send + 'static,
        create_session: fn(
            conn: PeerConnection,
            data_channel: DataChannel,
            host: bool,
            socket: T,
        ) -> TSession,
        room_name: String,
    ) -> Self
    where
        T: SignalingSocket + Send + 'static,
    {
        let (error_tx, error_rx) = mpsc::channel(1);
        let (session_tx, session_rx) = oneshot::channel();
        let (abort_tx, abort_rx) = watch::channel(false);

        let handle = {
            let room_name = room_name.clone();
            TOKIO_RUNTIME.spawn(async move {
                let origin = if cfg!(debug_assertions) {
                    "https://qayvs4nki2nl72kf4tn5h5yati0maxpe.lambda-url.ap-northeast-1.on.aws"
                } else {
                    "https://wxvo3rgklveqwyig4b3q5qupbq0mgvik.lambda-url.ap-northeast-1.on.aws"
                };
                let mut socket = create_socket(origin.into(), room_name, abort_rx.clone());
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
                let session = create_session(conn, dc, host, socket);
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
}

impl WaitingForOpponentInSharedRoom {
    pub fn new(room_name: String) -> Self {
        Self::internal_new(
            SignalingServerSharedRoomOpponentSocket::new,
            |pc, dc, host, _socket| BattleSession::new(pc, dc, host),
            room_name,
        )
    }
}

impl WaitingForOpponentInReservedRoom {
    pub fn new(room_name: String) -> Self {
        Self::internal_new(
            SignalingServerReservedRoomOpponentSocket::new,
            |conn, dc, host, socket| {
                (
                    BattleSession::new(conn, dc, host),
                    socket.into_key().map(RoomKey),
                )
            },
            room_name,
        )
    }

    pub fn try_into_session_and_waiting_for_spectator(
        mut self,
    ) -> Result<(BattleSession, WaitingForSpectator), Self> {
        let Ok((session, key)) = self.session_rx.try_recv() else {
            return Err(self);
        };
        let waiting = if let Some(key) = key {
            let waiting = WaitingForSpectatorInReservedRoom::new(self.room_name.clone(), key.0);
            WaitingForSpectator::ReservedRoom(waiting)
        } else {
            WaitingForSpectator::PureP2p(WaitingForPureP2pSpectator::standby())
        };
        Ok((session, waiting))
    }
}

impl WaitingForSpectatorInReservedRoom {
    pub fn new(room_name: String, key: String) -> Self {
        Self::internal_new(
            |origin, room_name, abort_rx| {
                SignalingServerReservedRoomSpectatorHostSocket::new(
                    origin, room_name, key, abort_rx,
                )
            },
            |conn, dc, _host, socket| {
                (
                    SpectatorHostSession::new(conn, dc),
                    RoomKey(socket.into_key()),
                )
            },
            room_name,
        )
    }

    pub fn try_session_and_waiting_for_spectator(
        &mut self,
    ) -> Result<(SpectatorHostSession, WaitingForSpectator), TryRecvError> {
        let (session, key) = self.session_rx.try_recv()?;
        let waiting = WaitingForSpectatorInReservedRoom::new(self.room_name.clone(), key.0);
        let waiting = WaitingForSpectator::ReservedRoom(waiting);
        Ok((session, waiting))
    }
}

impl WaitingForSpectatorHostInReservedRoom {
    pub fn new(room_name: String) -> Self {
        Self::internal_new(
            |origin, room_name, abort_rx| {
                SignalingServerReservedRoomSpectatorSocket::new(origin, room_name, abort_rx)
            },
            |pc, dc, host, _socket| {
                assert!(!host);
                SpectatorSession::new(pc, dc)
            },
            room_name,
        )
    }
}

impl<TSession> WaitingInRoom<TSession> {
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

impl<TSession> Drop for WaitingInRoom<TSession> {
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
