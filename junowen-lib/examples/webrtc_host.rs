use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use tokio::{join, net::windows::named_pipe, spawn};

use junowen_lib::{
    lang::Lang,
    signaling::{
        client::{receive_signaling, PeerConnectionImpl, StdioMockConnection},
        server::serve_signaling,
        transfer::{create_local_transfer, AsyncReadWriteTransfer},
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let name = r"\\.\pipe\webrtc-sandbox-host";
    let server_pipe = named_pipe::ServerOptions::new().create(name).unwrap();
    let client_pipe = named_pipe::ClientOptions::new().open(name).unwrap();
    server_pipe.connect().await?;

    let mut server_transfer = AsyncReadWriteTransfer::new(server_pipe);
    let mut client_transfer = AsyncReadWriteTransfer::new(client_pipe);

    let (mut server_to_host, mut host) = create_local_transfer();
    let server = spawn(async move {
        serve_signaling(&mut host, &mut server_transfer)
            .await
            .unwrap();
        server_transfer
    });
    let host = spawn(async move {
        let mut conn = PeerConnectionImpl::new().await.unwrap();
        receive_signaling(&mut server_to_host, &mut conn)
            .await
            .unwrap();
        let dc = conn
            .wait_for_open_data_channel(Duration::from_secs(10))
            .await
            .unwrap();
        (conn, dc)
    });
    let guest = spawn(async move {
        let lang = Lang::new("ja");
        let mut conn = StdioMockConnection::new(&lang);
        receive_signaling(&mut client_transfer, &mut conn)
            .await
            .unwrap();
        client_transfer
    });
    let (server, host, guest) = join!(server, host, guest);
    let _server_pipe = server.unwrap().into_inner();
    let (_conn, mut dc) = host.unwrap();
    let _client_pipe = guest.unwrap().into_inner();

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
