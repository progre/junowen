use std::sync::mpsc;

use anyhow::Result;
use junowen_lib::connection::signaling::socket::{AsyncReadWriteSocket, SignalingSocket};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe,
};
use tracing::error;

use crate::cui::{IpcMessageToCui, IpcMessageToHook};
use crate::session::{create_session, Session};

async fn ipc(session_sender: &mpsc::Sender<Session>) -> Result<()> {
    // NOTE: NamedPipeServer isn't reusable.
    let mut pipe = named_pipe::ServerOptions::new().create(r"\\.\pipe\junowen")?;
    pipe.connect().await?;
    pipe.write_all(&rmp_serde::to_vec(&IpcMessageToCui::Version(
        env!("CARGO_PKG_VERSION").to_owned(),
    ))?)
    .await?;

    let (anserer, conn) = AsyncReadWriteSocket::new(&mut pipe)
        .receive_signaling()
        .await?;
    let host = !anserer;
    pipe.write_all(&rmp_serde::to_vec(&IpcMessageToCui::Connected).unwrap())
        .await?;

    let delay = if host {
        let mut buf = [0u8; 4096];
        let len = pipe.read(&mut buf).await?;
        let msg: IpcMessageToHook = rmp_serde::from_slice(&buf[..len])?;
        let IpcMessageToHook::Delay(delay) = msg;
        Some(delay)
    } else {
        None
    };
    let session = create_session(conn, delay).await?;
    let mut closed = session.subscribe_closed_receiver();
    session_sender.send(session)?;
    closed.recv().await.unwrap();
    pipe.write_all(&rmp_serde::to_vec(&IpcMessageToCui::Disconnected).unwrap())
        .await?;
    Ok(())
}

pub fn init_interprocess(session_sender: mpsc::Sender<Session>) {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                loop {
                    match ipc(&session_sender).await {
                        Ok(ok) => ok,
                        Err(err) => {
                            error!("session aborted: {}", err);
                            continue;
                        }
                    };
                }
            });
    });
}
