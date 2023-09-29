use std::sync::mpsc;

use anyhow::Result;
use tokio::net::windows::named_pipe;

async fn ipc(session_sender: &mpsc::Sender<()>) -> Result<()> {
    // NOTE: NamedPipeServer isn't reusable.
    let pipe = named_pipe::ServerOptions::new().create(r"\\.\pipe\junowen")?;
    pipe.connect().await?;
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
