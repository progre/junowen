use anyhow::Error;
use getset::{CopyGetters, Getters, MutGetters};
use junowen_lib::connection::{
    signaling::{
        socket::{
            async_read_write_socket::SignalingServerMessage, channel_socket::ChannelSocket,
            SignalingSocket,
        },
        CompressedSdp,
    },
    DataChannel, PeerConnection,
};
use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::TOKIO_RUNTIME;

#[derive(CopyGetters, Getters, MutGetters)]
pub struct Signaling {
    offer_rx: oneshot::Receiver<CompressedSdp>,
    answer_rx: oneshot::Receiver<CompressedSdp>,
    #[get_mut = "pub"]
    msg_tx: Option<oneshot::Sender<SignalingServerMessage>>,
    #[get = "pub"]
    offer: Option<CompressedSdp>,
    #[get = "pub"]
    answer: Option<CompressedSdp>,
    error_rx: oneshot::Receiver<Error>,
    #[get = "pub"]
    error: Option<Error>,
    connected_rx: oneshot::Receiver<()>,
    #[get_copy = "pub"]
    connected: bool,
}

impl Signaling {
    pub fn new<T>(
        session_tx: mpsc::Sender<T>,
        create_session: fn(PeerConnection, DataChannel) -> T,
    ) -> Self
    where
        T: Send + 'static,
    {
        let (offer_tx, offer_rx) = oneshot::channel();
        let (answer_tx, answer_rx) = oneshot::channel();
        let (msg_tx, msg_rx) = oneshot::channel();
        let (error_tx, error_rx) = oneshot::channel();
        let (connected_tx, connected_rx) = oneshot::channel();
        TOKIO_RUNTIME.spawn(async move {
            let mut socket = ChannelSocket::new(offer_tx, answer_tx, msg_rx);
            let (conn, dc, _host) = match socket.receive_signaling().await {
                Ok(ok) => ok,
                Err(err) => {
                    info!("Signaling failed: {}", err);
                    let _ = error_tx.send(err);
                    return;
                }
            };
            tracing::trace!("signaling connected");
            session_tx.send(create_session(conn, dc)).await.unwrap();
            connected_tx.send(()).unwrap();
        });
        Self {
            offer_rx,
            answer_rx,
            msg_tx: Some(msg_tx),
            offer: None,
            answer: None,
            error_rx,
            error: None,
            connected_rx,
            connected: false,
        }
    }

    pub fn recv(&mut self) {
        if let Ok(offer) = self.offer_rx.try_recv() {
            self.offer = Some(offer);
        }
        if let Ok(answer) = self.answer_rx.try_recv() {
            self.answer = Some(answer);
        }
        if let Ok(error) = self.error_rx.try_recv() {
            self.error = Some(error);
        }
        if self.connected_rx.try_recv().is_ok() {
            self.connected = true;
        }
    }
}
