use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use junowen_lib::connection::signaling::{
    CompressedSdp,
    socket::{AsyncReadWriteSocket, DEFAULT_STUN_SERVER_URL, OfferResponse, SignalingSocket},
};
use tokio::{net::TcpStream, sync::watch};
use tracing::info;

/// TCP シグナリングの接続側。
/// [`super::tcp_signaling_host_socket::TcpSignalingHostSocket`] に接続し、answerer となる。
pub struct TcpSignalingGuestSocket {
    address: String,
    offline: bool,
    inner: Option<AsyncReadWriteSocket<TcpStream>>,
    abort_rx: watch::Receiver<bool>,
}

impl TcpSignalingGuestSocket {
    pub fn new(address: String, offline: bool, abort_rx: watch::Receiver<bool>) -> Self {
        Self {
            address,
            offline,
            inner: None,
            abort_rx,
        }
    }

    async fn connect(&mut self) -> Result<TcpStream> {
        let mut abort_rx = self.abort_rx.clone();
        let address = self.address.clone();
        let stream = tokio::select! {
            result = TcpStream::connect(&address) => {
                result.with_context(|| format!("Failed to connect to {}", address))?
            }
            _ = abort_rx.wait_for(|&val| val) => bail!("abort"),
        };
        info!("connected to {}", address);
        Ok(stream)
    }
}

#[async_trait]
impl SignalingSocket for TcpSignalingGuestSocket {
    fn timeout() -> Duration {
        Duration::from_secs(20 * 60)
    }

    fn ice_server_urls(&self) -> Vec<String> {
        if self.offline {
            vec![]
        } else {
            vec![DEFAULT_STUN_SERVER_URL.to_owned()]
        }
    }

    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        let stream = self.connect().await?;
        // 再試行のたびに接続し直すため、ここで作り直す
        let inner = self.inner.insert(AsyncReadWriteSocket::new(stream));
        inner.offer(desc).await
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        self.inner.as_mut().unwrap().answer(desc).await
    }
}
