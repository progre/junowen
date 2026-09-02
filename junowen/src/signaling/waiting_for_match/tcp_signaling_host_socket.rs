use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use junowen_lib::connection::signaling::{
    CompressedSdp,
    socket::{AsyncReadWriteServerSocket, DEFAULT_STUN_SERVER_URL, OfferResponse, SignalingSocket},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
};
use tracing::info;

/// 待ち受け側は設定値のうちポートのみを使う
fn parse_port(address: &str) -> Result<u16> {
    let (_, port) = address
        .rsplit_once(':')
        .with_context(|| format!("Port is not specified: {}", address))?;
    port.parse()
        .with_context(|| format!("Invalid port: {}", port))
}

/// TCP シグナリングの待ち受け側。接続してきた相手に対し、常に自身が offerer となる。
pub struct TcpSignalingHostSocket {
    address: String,
    offline: bool,
    listener: Option<TcpListener>,
    abort_rx: watch::Receiver<bool>,
}

impl TcpSignalingHostSocket {
    pub fn new(address: String, offline: bool, abort_rx: watch::Receiver<bool>) -> Self {
        Self {
            address,
            offline,
            listener: None,
            abort_rx,
        }
    }

    async fn listener(&mut self) -> Result<&TcpListener> {
        if self.listener.is_none() {
            let port = parse_port(&self.address)?;
            // 相手がどの経路から来るか分からないため、全アドレスで待ち受ける
            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            let listener = TcpListener::bind(addr)
                .await
                .with_context(|| format!("Failed to listen on {}", addr))?;
            info!("listening on {}", addr);
            self.listener = Some(listener);
        }
        Ok(self.listener.as_ref().unwrap())
    }

    async fn accept(&mut self) -> Result<TcpStream> {
        let mut abort_rx = self.abort_rx.clone();
        let listener = self.listener().await?;
        let (stream, peer_addr) = tokio::select! {
            result = listener.accept() => result?,
            _ = abort_rx.wait_for(|&val| val) => bail!("abort"),
        };
        info!("accepted from {}", peer_addr);
        Ok(stream)
    }
}

#[async_trait]
impl SignalingSocket for TcpSignalingHostSocket {
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
        let stream = self.accept().await?;
        AsyncReadWriteServerSocket::new(stream).offer(desc).await
    }

    async fn answer(&mut self, _desc: CompressedSdp) -> Result<()> {
        unreachable!()
    }
}
