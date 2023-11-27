use std::time::Duration;

use anyhow::{bail, Result};

use junowen_lib::signaling_server::room::DeleteRoomRequestBody;
use reqwest::{header::RETRY_AFTER, Response};
use tokio::{sync::watch, time::sleep};
use tracing::info;

pub fn retry_after(res: &Response) -> Option<u32> {
    res.headers()
        .get(RETRY_AFTER)
        .and_then(|x| x.to_str().ok())
        .and_then(|x| x.parse::<u32>().ok())
}

async fn delete_room(
    client: &reqwest::Client,
    url: &str,
    body: &DeleteRoomRequestBody,
) -> Result<()> {
    info!("DELETE {}", url);
    let res = client.delete(url).json(body).send().await?;
    info!("{:?}", res.status());
    Ok(())
}

pub async fn sleep_or_abort_and_delete_room(
    retry_after: u32,
    abort_rx: &mut watch::Receiver<bool>,
    client: &reqwest::Client,
    url_delete_room: &str,
    key: &str,
) -> Result<()> {
    let task1 = sleep(Duration::from_secs(retry_after as u64));
    let task2 = abort_rx.wait_for(|&val| val);
    tokio::select! {
        _ = task1 => return Ok(()),
        _ = task2 => {},
    };
    let body = DeleteRoomRequestBody::new(key.to_owned());
    delete_room(client, url_delete_room, &body).await?;
    bail!("abort");
}
