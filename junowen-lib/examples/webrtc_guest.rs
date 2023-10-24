use anyhow::Result;
use bytes::Bytes;
use tokio::{net::windows::named_pipe, spawn};

use junowen_lib::{
    connection::signaling::{
        socket::{AsyncReadWriteSocket, SignalingSocket},
        stdio_signaling_interface::connect_as_answerer,
    },
    lang::Lang,
};

#[tokio::main]
async fn main() -> Result<()> {
    let name = &format!(r"\\.\pipe\{}", env!("CARGO_PKG_NAME"));
    let server_pipe = named_pipe::ServerOptions::new().create(name).unwrap();
    let mut client_pipe = named_pipe::ClientOptions::new().open(name).unwrap();
    server_pipe.connect().await?;

    let task = spawn(async move {
        let mut socket = AsyncReadWriteSocket::new(server_pipe);
        socket.receive_signaling(false).await.unwrap()
    });
    connect_as_answerer(&mut client_pipe, &Lang::resolve())
        .await
        .unwrap();
    let (_conn, mut dc) = task.await.unwrap();

    let msg = dc.recv().await.unwrap();
    println!("msg: {:?}", msg);
    dc.message_sender
        .send(Bytes::from_iter(b"pong".iter().copied()))
        .await?;
    let msg = dc.recv().await.unwrap();
    println!("msg: {:?}", msg);
    dc.message_sender
        .send(Bytes::from_iter(b"pong".iter().copied()))
        .await?;
    let msg = dc.recv().await;
    println!("msg: {:?}", msg);

    Ok(())
}
