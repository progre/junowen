pub mod battle;
mod delayed_inputs;
pub mod spectator;
pub mod spectator_host;

use std::sync::mpsc;

use anyhow::Result;
use bytes::Bytes;
use junowen_lib::connection::DataChannel;
use rmp_serde::decode::Error;
use serde::{Deserialize, Serialize};
use tokio::spawn;
use tracing::debug;

#[derive(Debug, Deserialize, Serialize)]
pub struct RoundInitial {
    pub seed1: u16,
    pub seed2: u16,
    pub seed3: u16,
    pub seed4: u16,
}

fn to_channel<T>(
    mut data_channel: DataChannel,
    decode: fn(input: &[u8]) -> Result<T, Error>,
) -> (mpsc::Sender<T>, mpsc::Receiver<T>)
where
    T: Serialize + Send + 'static,
{
    let (hook_outgoing_tx, hook_outgoing_rx) = std::sync::mpsc::channel();
    let data_channel_message_sender = data_channel.message_sender.clone();

    spawn(async move {
        let mut hook_outgoing_rx = hook_outgoing_rx;
        loop {
            let (msg, reusable) =
                tokio::task::spawn_blocking(move || (hook_outgoing_rx.recv(), hook_outgoing_rx))
                    .await
                    .unwrap();
            let msg = match msg {
                Ok(ok) => ok,
                Err(err) => {
                    debug!("recv hook outgoing msg error: {}", err);
                    return;
                }
            };
            hook_outgoing_rx = reusable;
            let data = Bytes::from(rmp_serde::to_vec(&msg).unwrap());
            if let Err(err) = data_channel_message_sender.send(data).await {
                debug!("send hook outgoing msg error: {}", err);
                return;
            }
        }
    });

    let (hook_incoming_tx, hook_incoming_rx) = mpsc::channel();
    spawn(async move {
        loop {
            let Some(data) = data_channel.recv().await else {
                return;
            };
            let msg = decode(&data).unwrap();
            if let Err(err) = hook_incoming_tx.send(msg) {
                debug!("send hook incoming msg error: {}", err);
                return;
            }
        }
    });
    (hook_outgoing_tx, hook_incoming_rx)
}
