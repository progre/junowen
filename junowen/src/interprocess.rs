use anyhow::Result;
use tokio::{io::AsyncWriteExt, net::windows::named_pipe};
use tracing::error;

use crate::cui::IpcMessageToCui;

async fn ipc() -> Result<()> {
    // NOTE: NamedPipeServer isn't reusable.
    let mut pipe = named_pipe::ServerOptions::new().create(r"\\.\pipe\junowen")?;
    pipe.connect().await?;
    pipe.write_all(&rmp_serde::to_vec(&IpcMessageToCui::Version(
        env!("CARGO_PKG_VERSION").to_owned(),
    ))?)
    .await?;
    let _ = pipe.disconnect();
    Ok(())
}

pub fn init_interprocess() {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                loop {
                    match ipc().await {
                        Ok(ok) => ok,
                        Err(err) => {
                            error!("ipc aborted: {}", err);
                            continue;
                        }
                    };
                }
            });
    });
}
