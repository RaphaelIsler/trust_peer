use super::WebRtcIds;
use crate::ledger_node::backend::con::Service as ToLedgerNode;
use anyhow::Result;
use core_types::Timestamp;
use p2p_webrtc::data_channel::DataChannel;
use p2p_webrtc::{P2pConfig, P2pWebRtc};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub enum Id {
    Established(blockchain::blockchain::Id),
    Creating(u8),
}

#[derive(helper::ServiceWrapper)]
pub enum ToConnection {
    Stop { tx: oneshot::Sender<Result<()>> },
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum P2P {
    Hello,
    IAm {
        private: blockchain::Id,
        public: blockchain::Id,
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        birthday: Option<chrono::NaiveDate>,
    },
}

#[derive(Clone)]
pub struct Service {
    tx: mpsc::Sender<ToConnection>,
}

pub struct Internal {
    id: Id,
    data_channel: DataChannel,
    to_ledger: ToLedgerNode,
    state: Option<super::TrustState>,
}

impl PartialEq for Service {
    fn eq(&self, other: &Self) -> bool {
        self.tx.same_channel(&other.tx)
    }
}

impl Service {
    const DEFAULT_SIGNALING_SERVER: &'static str = "ws://127.0.0.1:3000";
    pub fn start_init(id: u8, web_rtc: WebRtcIds, to_ledger: ToLedgerNode) -> Self {
        Self::start(Id::Creating(id), web_rtc, to_ledger)
    }

    pub fn restart(
        id: blockchain::blockchain::Id,
        web_rtc: WebRtcIds,
        to_ledger: ToLedgerNode,
    ) -> Self {
        Self::start(Id::Established(id), web_rtc, to_ledger)
    }

    fn start(id: Id, web_rtc: WebRtcIds, to_ledger: ToLedgerNode) -> Self {
        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(async move {
            let config =
                P2pConfig::new(Self::DEFAULT_SIGNALING_SERVER.to_string(), web_rtc.room_id)
                    .with_peer_id(web_rtc.own_id)
                    .with_timeout(30);

            to_ledger.connection_state(id.clone(), "Connecting over Signaling Server", 0.0).await;

            let mut p2p = P2pWebRtc::new(config);
            let data_channel = match p2p.connect().await {
                Ok(data_channel) => data_channel,
                Err(error) => {
                    let _ = to_ledger.failed(id.clone(), error).await;
                    return Ok::<(), anyhow::Error>(());
                }
            };
            to_ledger.connection_state(id.clone(), "Connected to other Node", 0.1).await;

            let mut internal = Internal {
                id: id,
                data_channel: data_channel,
                to_ledger: to_ledger.clone(),
                state: None,
            };
            internal.process(to_ledger, rx).await;
            Ok(())
        });

        Self { tx }
    }
}

impl Internal {
    async fn handle_ledger(&mut self, msg: ToConnection, to_ledger: &ToLedgerNode) {
        match msg {
            ToConnection::Stop { tx } => {
                let close_result = self.data_channel.close().await;
                let _ = to_ledger.stopped(self.id.clone()).await;
                let _ = tx.send(close_result);
            }
        }
    }

    async fn handle_msg(&mut self, msg: P2P) {
        log::info!("Received message from other node: {:?}", serde_json::to_string(&msg));
        match msg {
            P2P::Hello => {
                self.to_ledger.connection_state(self.id.clone(), "Hello Sayed", 0.2).await;
//                if self.state.is_none() {
//                }
            }
            P2P::IAm {
                private,
                public,
                first_name,
                last_name,
                middle_name,
                birthday,
            } => {
                if self.state.is_none() {
                    self.state = Some(super::TrustState {
                        id: public,
                        last_public_singleton_checked: None,
                        last_validated_public_block_id: None,
                        name_accepted: None,
                        last_seen_at: Timestamp::now(),
                        private_chain_id: private.clone(),
                    });
                    let _ = self
                        .to_ledger
                        .name_accept_required(
                            public,
                            private,
                            &first_name,
                            &last_name,
                            middle_name,
                            birthday,
                        )
                        .await;
                }
            }
        }
    }

    async fn send(&self, msg: P2P) {
        let data = serde_json::to_vec(&msg).unwrap();
        self.data_channel.send(&data).await.unwrap_or_else(|error| {
            let _ = self
                .to_ledger
                .failed(self.id.clone(), anyhow::Error::msg(error.to_string()));
        });
    }

    async fn process(&mut self, to_ledger: ToLedgerNode, mut rx: mpsc::Receiver<ToConnection>) {
        let poll_interval = Duration::from_millis(50);
        self.send(P2P::Hello).await;
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    self.handle_ledger(msg, &to_ledger).await
                }
                _ = tokio::time::sleep(poll_interval) => {
                    if !self.data_channel.is_open() {
                        let _ = to_ledger.stopped(self.id.clone());
                        break;
                    }

                    if let Some(data) = self.data_channel.try_recv().await {
                        let mgs: P2P = serde_json::from_slice(&data).unwrap();
                        self.handle_msg(mgs).await;
                    }
                }
                else => {
                    break;
                }
            }
        }
    }
}
