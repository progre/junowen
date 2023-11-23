use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;

use junowen_lib::{
    connection::signaling::{
        socket::{OfferResponse, SignalingSocket},
        CompressedSdp,
    },
    signaling_server::custom::{
        DeleteRoomRequestBody, PostRoomJoinRequestBody, PostRoomJoinResponse,
        PostRoomKeepRequestBody, PostRoomKeepResponse, PutRoomRequestBody, PutRoomResponse,
    },
};
use reqwest::{header::RETRY_AFTER, Response};
use tokio::{sync::watch, time::sleep};
use tracing::info;

fn retry_after(res: &Response) -> Option<u32> {
    res.headers()
        .get(RETRY_AFTER)
        .and_then(|x| x.to_str().ok())
        .and_then(|x| x.parse::<u32>().ok())
}

pub struct SignalingServerSocket {
    client: reqwest::Client,
    origin: String,
    room_name: String,
    abort_rx: watch::Receiver<bool>,
}

impl SignalingServerSocket {
    pub fn new(origin: String, room_name: String, abort_rx: watch::Receiver<bool>) -> Self {
        Self {
            client: reqwest::Client::new(),
            origin,
            room_name,
            abort_rx,
        }
    }

    pub fn into_inner(self) -> (String, String) {
        (self.origin, self.room_name)
    }

    async fn delete(&self, body: &DeleteRoomRequestBody) -> Result<()> {
        let url = format!("{}/custom/{}", self.origin, self.room_name);
        info!("DELETE {}", url);
        let res = self.client.delete(url).json(body).send().await?;
        info!("{:?}", res.status());
        Ok(())
    }

    async fn sleep_or_delete(
        &mut self,
        body: &DeleteRoomRequestBody,
        retry_after: u32,
    ) -> Result<()> {
        let task1 = sleep(Duration::from_secs(retry_after as u64));
        let task2 = self.abort_rx.wait_for(|&val| val);
        tokio::select! {
            _ = task1 => return Ok(()),
            _ = task2 => {},
        };
        self.delete(body).await?;
        bail!("abort");
    }
}

#[async_trait]
impl SignalingSocket for SignalingServerSocket {
    fn timeout() -> Duration {
        Duration::from_secs(10)
    }

    async fn offer(&mut self, desc: CompressedSdp) -> Result<OfferResponse> {
        let url = format!("{}/custom/{}", self.origin, self.room_name);
        info!("PUT {}", url);
        let res = self
            .client
            .put(url)
            .json(&PutRoomRequestBody::new(desc))
            .send()
            .await?;
        let res =
            PutRoomResponse::parse(res.status(), retry_after(&res), &res.text().await.unwrap())?;
        info!("{:?}", res);
        let (delete_req_body, key) = match res {
            PutRoomResponse::Conflict { body, .. } => {
                return Ok(OfferResponse::Offer(body.into_offer()))
            }
            PutRoomResponse::CreatedWithAnswer { body, .. } => {
                return Ok(OfferResponse::Answer(body.into_answer()));
            }
            PutRoomResponse::CreatedWithKey { retry_after, body } => {
                let key = body.into_key();
                let delete_req_body = DeleteRoomRequestBody::new(key.clone());
                self.sleep_or_delete(&delete_req_body, retry_after).await?;
                (delete_req_body, key)
            }
        };

        let keep_req_body = PostRoomKeepRequestBody::new(key);
        loop {
            let url = format!("{}/custom/{}/keep", self.origin, self.room_name);
            info!("POST {}", url);
            let res = self.client.post(url).json(&keep_req_body).send().await?;
            let res = PostRoomKeepResponse::parse(
                res.status(),
                retry_after(&res),
                res.text().await.as_ref().map(|x| x.as_str()).ok(),
            )?;
            info!("{:?}", res);
            match res {
                PostRoomKeepResponse::BadRequest => {
                    bail!("bad request")
                }
                PostRoomKeepResponse::Ok(body) => {
                    return Ok(OfferResponse::Answer(body.into_answer()));
                }
                PostRoomKeepResponse::NoContent { retry_after } => {
                    self.sleep_or_delete(&delete_req_body, retry_after).await?;
                }
            }
        }
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        let res = self
            .client
            .post(format!("{}/custom/{}/join", self.origin, self.room_name))
            .json(&PostRoomJoinRequestBody::new(desc))
            .send()
            .await?;
        let res = PostRoomJoinResponse::parse(res.status())?;
        match res {
            PostRoomJoinResponse::Ok => Ok(()),
            PostRoomJoinResponse::Conflict => bail!("room is full"),
        }
    }
}
