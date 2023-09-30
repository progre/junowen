use std::sync::mpsc;

use anyhow::Result;
use junowen_lib::session::connection::signaling::socket::{AsyncReadWriteSocket, SignalingSocket};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe,
};

use crate::cui::{IpcMessageToCui, IpcMessageToHook};

async fn ipc(session_sender: &mpsc::Sender<()>) -> Result<()> {
    // NOTE: NamedPipeServer isn't reusable.
    let mut pipe = named_pipe::ServerOptions::new().create(r"\\.\pipe\junowen")?;
    pipe.connect().await?;
    pipe.write_all(
        &rmp_serde::to_vec(&IpcMessageToCui::Version(
            env!("CARGO_PKG_VERSION").to_owned(),
        ))
        .unwrap(),
    )
    .await?;

    let (anserer, _conn) = AsyncReadWriteSocket::new(&mut pipe)
        .receive_signaling()
        .await?;
    let host = !anserer;
    pipe.write_all(&rmp_serde::to_vec(&IpcMessageToCui::Connected).unwrap())
        .await?;

    let _delay = if host {
        let mut buf = [0u8; 4096];
        let len = pipe.read(&mut buf).await?;
        let msg: IpcMessageToHook = rmp_serde::from_slice(&buf[..len])?;
        let IpcMessageToHook::Delay(delay) = msg;
        Some(delay)
    } else {
        None
    };
    session_sender.send(())?;
    Ok(())
}

pub fn init_interprocess(session_sender: mpsc::Sender<()>) {
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
                            eprintln!("{}", err);
                            continue;
                        }
                    };
                }
            });
    });
}
