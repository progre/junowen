use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use tokio::{
    spawn,
    sync::{mpsc, oneshot},
};
use webrtc::data_channel::RTCDataChannel;

pub struct DataChannel {
    rtc: Arc<RTCDataChannel>,
    open_rx: Option<oneshot::Receiver<()>>,
    close_rx: mpsc::Receiver<()>,
    pc_disconnected_rx: mpsc::Receiver<()>,
    pub message_sender: mpsc::Sender<Bytes>,
    incoming_message_rx: mpsc::Receiver<Bytes>,
}

impl DataChannel {
    pub async fn new(rtc: Arc<RTCDataChannel>, pc_disconnected_rx: mpsc::Receiver<()>) -> Self {
        let (open_tx, open_rx) = oneshot::channel();
        let mut open_tx = Some(open_tx);
        let (message_sender, mut outgoing_message_receiver) = mpsc::channel(1);
        let (incoming_message_tx, incoming_message_rx) = mpsc::channel(1);
        let (close_sender, close_receiver) = mpsc::channel(1);
        rtc.on_open(Box::new(move || {
            let open_sender = open_tx.take().unwrap();
            Box::pin(async move { open_sender.send(()).unwrap() })
        }));
        rtc.on_message(Box::new(move |msg| {
            let incoming_message_tx = incoming_message_tx.clone();
            Box::pin(async move { incoming_message_tx.send(msg.data).await.unwrap() })
        }));
        rtc.on_error(Box::new(|err| {
            eprintln!("{}", err);
            Box::pin(async {})
        }));
        rtc.on_close(Box::new(move || {
            let close_sender = close_sender.clone();
            Box::pin(async move {
                let _ = close_sender.send(()).await;
            })
        }));
        rtc.on_buffered_amount_low(Box::new(|| Box::pin(async {})))
            .await;

        {
            // NOTE: To make it possible to have separate references for receiving and sending,
            //       sending is implemented with a channel and a task.
            //       Or, it would be nice to have something like tokio::io::{ReadHalf, WriteHalf}.
            let rtc = rtc.clone();
            spawn(async move {
                while let Some(data) = outgoing_message_receiver.recv().await {
                    let result = rtc.send(&data).await;
                    if let Err(webrtc::Error::ErrClosedPipe) = result {
                        return;
                    } else if let Err(err) = result {
                        eprintln!("{}", err);
                    }
                }
            });
        }

        Self {
            rtc,
            open_rx: Some(open_rx),
            message_sender,
            incoming_message_rx,
            close_rx: close_receiver,
            pc_disconnected_rx,
        }
    }

    pub async fn wait_for_open_data_channel(&mut self) {
        self.open_rx.take().unwrap().await.unwrap()
    }

    /// This method returns `None` if either `incoming_message_rx`,
    /// `RTCDataChannel`, or `RTCPeerConnection` is closed.
    pub async fn recv(&mut self) -> Option<Bytes> {
        tokio::select! {
            result = self.incoming_message_rx.recv() => result,
            _ = self.close_rx.recv() => None,
            _ = self.pc_disconnected_rx.recv() => None,
        }
    }

    pub async fn close(self) -> Result<()> {
        Ok(self.rtc.close().await?)
    }
}
