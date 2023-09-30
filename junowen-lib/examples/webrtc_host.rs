use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use tokio::{net::windows::named_pipe, spawn};

use junowen_lib::{
    lang::Lang,
    signaling::{
        client::{receive_signaling, PeerConnection},
        socket::AsyncReadWriteSocket,
        stdio_signaling_interface::signaling_as_offerer,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let name = &format!(r"\\.\pipe\{}", env!("CARGO_PKG_NAME"));
    let server_pipe = named_pipe::ServerOptions::new().create(name).unwrap();
    let mut client_pipe = named_pipe::ClientOptions::new().open(name).unwrap();
    server_pipe.connect().await?;

    let task = spawn(async move {
        let mut socket = AsyncReadWriteSocket::new(server_pipe);
        let mut conn = PeerConnection::new().await.unwrap();
        receive_signaling(&mut socket, &mut conn).await.unwrap();
        let dc = conn
            .wait_for_open_data_channel(Duration::from_secs(10))
            .await
            .unwrap();
        (conn, dc)
    });
    signaling_as_offerer(&mut client_pipe, &Lang::new("ja"))
        .await
        .unwrap();
    let (_conn, mut dc) = task.await.unwrap();

    dc.message_sender
        .send(Bytes::from_iter(b"ping".iter().copied()))
        .await?;
    let msg = dc.recv().await.unwrap();
    println!("msg: {:?}", msg);
    dc.message_sender
        .send(Bytes::from_iter(b"bye".iter().copied()))
        .await?;
    dc.close().await?;
    Ok(())
}
