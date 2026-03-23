use crate::{ToBackend, ToFrontend};
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[component]
pub fn App() -> Element {
    let tx_backend = use_context::<mpsc::Sender<ToBackend>>();
    let rx_arc = use_context::<Arc<Mutex<Option<mpsc::Receiver<ToFrontend>>>>>();

    let ledger_nodes = use_context_provider(|| {
        Signal::new(HashMap::<String, crate::ledger_node::frontend::Frontend>::new())
    });
    let tx = tx_backend.clone();

    use_effect(move || {
        let rx_frontend = rx_arc.lock().unwrap().take().expect("already taken");
        let tx_backend = tx.clone();
        spawn(async move {
            let mut service = Service::new_local(tx_backend, rx_frontend, ledger_nodes);
            service.run().await;
        });
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { class: "app-container",
            h1 { "TrustPeer" }
            if ledger_nodes().is_empty() {
                crate::ledger_node::identification::Create {
                    on_create: move |ident: crate::ledger_node::Identification| {
                        let tx = tx_backend.clone();
                        spawn(async move {
                            let _ = tx.send(ToBackend::Create(ident)).await;
                        });
                    },
                    on_error: move |_| {},
                }
            } else {
                for (private_str , store) in ledger_nodes() {
                    crate::ledger_node::frontend::Overview {
                        to_backend: {
                            let private_str = private_str.clone();
                            let tx = tx_backend.clone();
                            move |msg: crate::ledger_node::ToBackend| {
                                if let Ok(private) =
                                    blockchain::blockchain::Id::parse_str(&private_str)
                                {
                                    let tx = tx.clone();
                                    spawn(async move {
                                        let _ = tx
                                            .send(ToBackend::Ledger { private, msg })
                                            .await;
                                    });
                                }
                            }
                        },
                        store,
                    }
                }
            }
        }
    }
}

pub struct Service {
    rx_frontend: mpsc::Receiver<ToFrontend>,
    tx_backend: mpsc::Sender<ToBackend>,
    ledger_nodes: Signal<HashMap<String, crate::ledger_node::frontend::Frontend>>,
}

impl Service {
    pub fn new_local(
        tx_backend: mpsc::Sender<ToBackend>,
        rx_frontend: mpsc::Receiver<ToFrontend>,
        ledger_nodes: Signal<HashMap<String, crate::ledger_node::frontend::Frontend>>,
    ) -> Self {
        Self {
            rx_frontend,
            tx_backend,
            ledger_nodes,
        }
    }

    pub async fn run(&mut self) {
        log::info!("start loop");
        self.send_message(&ToBackend::Init).await;

        loop {
            if let Some(msg) = self.rx_frontend.recv().await {
                match msg {
                    ToFrontend::Init(entries) => {
                        let mut nodes = self.ledger_nodes.write();
                        for (private, _money) in &entries {
                            nodes
                                .entry(private.to_string())
                                .or_insert_with(crate::ledger_node::frontend::Frontend::new);
                        }
                    }
                    ToFrontend::Ledger { private, msg } => {
                        let mut nodes = self.ledger_nodes.write();
                        let node = nodes
                            .entry(private.to_string())
                            .or_insert_with(crate::ledger_node::frontend::Frontend::new);
                        node.handle_msg(msg);
                    }
                }
            }
        }
    }

    async fn send_message(&self, msg: &ToBackend) {
        let _ = self.tx_backend.send(msg.clone()).await;
    }
}
