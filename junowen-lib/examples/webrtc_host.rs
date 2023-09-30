use anyhow::Result;
use bytes::Bytes;
use tokio::{net::windows::named_pipe, spawn};

use junowen_lib::{
    lang::Lang,
    session::connection::signaling::{
        socket::{AsyncReadWriteSocket, SignalingSocket},
        stdio_signaling_interface::connect_as_offerer,
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
        socket.receive_signaling().await.unwrap()
    });
    connect_as_offerer(&mut client_pipe, &Lang::new("ja"))
        .await
        .unwrap();
    let (_, mut conn) = task.await.unwrap();

    conn.message_sender
        .send(Bytes::from_iter(b"ping".iter().copied()))
        .await?;
    let msg = conn.recv().await.unwrap();
    println!("msg: {:?}", msg);
    conn.message_sender
        .send(Bytes::from_iter(b"ping".iter().copied()))
        .await?;
    let msg = conn.recv().await.unwrap();
    println!("msg: {:?}", msg);
    conn.message_sender
        .send(Bytes::from_iter(b"bye".iter().copied()))
        .await?;
    conn.close().await?;
    Ok(())
}
