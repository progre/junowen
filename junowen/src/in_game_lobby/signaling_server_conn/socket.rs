use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;

use junowen_lib::{
    connection::signaling::{
        socket::{OfferResponse, SignalingSocket},
        CompressedSdp,
    },
    signaling_server::custom::{
        DeleteOfferRequestBody, PostAnswerRequestBody, PostAnswerResponse,
        PostOfferKeepRequestBody, PostOfferKeepResponse, PutOfferRequestBody, PutOfferResponse,
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

    async fn delete(&self, body: &DeleteOfferRequestBody) -> Result<()> {
        let url = format!("{}/custom/{}", self.origin, self.room_name);
        info!("DELETE {}", url);
        let res = self.client.delete(url).json(body).send().await?;
        info!("{:?}", res.status());
        Ok(())
    }

    async fn sleep_or_delete(
        &mut self,
        body: &DeleteOfferRequestBody,
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
            .json(&PutOfferRequestBody::new(desc))
            .send()
            .await?;
        let res =
            PutOfferResponse::parse(res.status(), retry_after(&res), &res.text().await.unwrap())?;
        info!("{:?}", res);
        let (delete_req_body, key) = match res {
            PutOfferResponse::Conflict { body, .. } => {
                return Ok(OfferResponse::Offer(body.into_offer()))
            }
            PutOfferResponse::CreatedWithAnswer { body, .. } => {
                return Ok(OfferResponse::Answer(body.into_answer()));
            }
            PutOfferResponse::CreatedWithKey { retry_after, body } => {
                let key = body.into_key();
                let delete_req_body = DeleteOfferRequestBody::new(key.clone());
                self.sleep_or_delete(&delete_req_body, retry_after).await?;
                (delete_req_body, key)
            }
        };

        let keep_req_body = PostOfferKeepRequestBody::new(key);
        loop {
            let url = format!("{}/custom/{}/keep", self.origin, self.room_name);
            info!("POST {}", url);
            let res = self.client.post(url).json(&keep_req_body).send().await?;
            let res = PostOfferKeepResponse::parse(
                res.status(),
                retry_after(&res),
                res.text().await.as_ref().map(|x| x.as_str()).ok(),
            )?;
            info!("{:?}", res);
            match res {
                PostOfferKeepResponse::BadRequest => {
                    bail!("bad request")
                }
                PostOfferKeepResponse::Ok(body) => {
                    return Ok(OfferResponse::Answer(body.into_answer()));
                }
                PostOfferKeepResponse::NoContent { retry_after } => {
                    self.sleep_or_delete(&delete_req_body, retry_after).await?;
                }
            }
        }
    }

    async fn answer(&mut self, desc: CompressedSdp) -> Result<()> {
        let res = self
            .client
            .post(format!("{}/custom/{}/join", self.origin, self.room_name))
            .json(&PostAnswerRequestBody::new(desc))
            .send()
            .await?;
        let res = PostAnswerResponse::parse(res.status())?;
        match res {
            PostAnswerResponse::Ok => Ok(()),
            PostAnswerResponse::Conflict => bail!("room is full"),
        }
    }
}
