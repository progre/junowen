use std::{time::Duration, u64::MAX};

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
    let name = r"\\.\pipe\webrtc-guest";
    let server_pipe = named_pipe::ServerOptions::new().create(name).unwrap();
    let client_pipe = named_pipe::ClientOptions::new().open(name).unwrap();
    server_pipe.connect().await?;

    let mut server_transfer = AsyncReadWriteTransfer::new(server_pipe);
    let mut client_transfer = AsyncReadWriteTransfer::new(client_pipe);

    let (mut server_to_guest, mut guest) = create_local_transfer();
    let server = spawn(async move {
        serve_signaling(&mut server_transfer, &mut guest)
            .await
            .unwrap();
        server_transfer
    });
    let host = spawn(async move {
        let lang = Lang::new("ja");
        let mut conn = StdioMockConnection::new(&lang);
        receive_signaling(&mut client_transfer, &mut conn)
            .await
            .unwrap();
        client_transfer
    });
    let guest = spawn(async move {
        let mut conn = PeerConnectionImpl::new().await.unwrap();
        receive_signaling(&mut server_to_guest, &mut conn)
            .await
            .unwrap();
        let dc = conn
            .wait_for_open_data_channel(Duration::from_secs(MAX))
            .await
            .unwrap();
        (conn, dc)
    });
    let (server, host, guest) = join!(server, host, guest);
    let _server_pipe = server.unwrap().into_inner();
    let _client_pipe = host.unwrap().into_inner();
    let (_conn, mut dc) = guest.unwrap();

    let msg = dc.recv().await.unwrap();
    println!("msg: {:?}", msg);
    dc.message_sender
        .send(Bytes::from_iter(b"pong".iter().copied()))
        .await?;
    let msg = dc.recv().await.unwrap();
    println!("msg: {:?}", msg);
    dc.close().await?;

    Ok(())
}
