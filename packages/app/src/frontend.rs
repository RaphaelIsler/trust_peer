use crate::user;
use crate::{ToBackend, ToFrontend};
use dioxus::prelude::*;
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[component]
pub fn App() -> Element {

    let tx_backend = use_context::<mpsc::Sender<ToBackend>>();
    let rx_arc = use_context::<Arc<Mutex<Option<mpsc::Receiver<ToFrontend>>>>>();

    let user_store = use_context_provider(|| Signal::new(user::Store::new()));
    let ledger_nodes = use_context_provider(|| Signal::new(HashMap::<String, crate::ledger_node::frontend::Frontend>::new()));
    let value = tx_backend.clone();
    //let channels = use:context_provider(|| AppProps { rx_frontend, tx_backend });
    use_effect(move || {
        let rx_frontend = rx_arc.lock().unwrap().take().expect("already taken");
        let tx_backend = value.clone();

        spawn(async move {
            let mut service = Service::new_local(tx_backend, rx_frontend, user_store, ledger_nodes);
            service.run().await;
        });
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { class: "app-container",
            h1 { "TrustPeer" }
            user::store::Overview {
                to_backend: move |msg: user::ToBackend| {
                    let tx = tx_backend.clone();
                    spawn(async move {
                        let _ = tx.send(msg.into()).await;
                    });
                },
                store: user_store(),
            }
        }
    }
}

pub struct Service {
    //    event_client: Option<EventClient>,
    rx_frontend: mpsc::Receiver<ToFrontend>,
    tx_backend: mpsc::Sender<ToBackend>,
    user_store: Signal<crate::user::Store>,
    ledger_nodes: Signal<HashMap<String, crate::ledger_node::frontend::Frontend>>,
    //    reconnect_timer: Option<Timeout>,
}

//#[derive(Clone)]
//struct MessageSender(futures_channel::mpsc::UnboundedSender<ToFrontend>);

impl Service {
    pub fn new_local(
        tx_backend: mpsc::Sender<ToBackend>,
        rx_frontend: mpsc::Receiver<ToFrontend>,
        user_store: Signal<crate::user::Store>,
        ledger_nodes: Signal<HashMap<String, crate::ledger_node::frontend::Frontend>>,
    ) -> Self {
        let ret = Self {
            //            event_client: None,
            rx_frontend,
            tx_backend,
            user_store,
            ledger_nodes,
            //          reconnect_timer: None,
        };
        //        ret.open_connection();
        ret
    }

    pub async fn run(&mut self) {
        // wait ready message from websocket
        //        let _ = self.rx.next().await;
        log::info!("start loop");
        self.send_message(&ToBackend::Init).await;

        loop {
            if let Some(msg) = self.rx_frontend.recv().await {
                match msg {
                    ToFrontend::Init(entries) => {
                        let users: Vec<crate::user::User> = entries.iter().map(|(u, _, _)| u.clone()).collect();
                        self.user_store.set(crate::user::Store::from(users));
                        let mut nodes = self.ledger_nodes.write();
                        for (_, private, _) in &entries {
                            nodes.entry(private.to_string()).or_insert_with(crate::ledger_node::frontend::Frontend::new);
                        }
                    }
                    ToFrontend::User(user) => match user {
                        crate::user::ToFrontend::Users(store) => {
                            self.user_store.set(store);
                        }
                    },
                    ToFrontend::Ledger { private, msg } => {
                        let mut nodes = self.ledger_nodes.write();
                        let node = nodes.entry(private.to_string()).or_insert_with(crate::ledger_node::frontend::Frontend::new);
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
