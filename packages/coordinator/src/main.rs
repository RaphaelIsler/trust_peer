use api::{FromCoordinator, ServerConfig, ToCoordinator};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

fn config_path_from_args() -> Option<std::path::PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                if let Some(path) = args.next() {
                    return Some(std::path::PathBuf::from(path));
                }
            }
            _ => {
                if !arg.starts_with('-') {
                    return Some(std::path::PathBuf::from(arg));
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config_path_from_args().unwrap_or_else(ServerConfig::config_path_from_env);
    let config = ServerConfig::load_or_create(&config_path)?;
    let addr = config.socket_addr();
    let listener = TcpListener::bind(&addr).await?;
    println!("Coordinator server listening on {addr}");

    loop {
        let (socket, peer) = listener.accept().await?;
        println!("Coordinator: connection from {peer}");
        tokio::spawn(async move {
            let ws_stream = match accept_async(socket).await {
                Ok(stream) => stream,
                Err(err) => {
                    eprintln!("Coordinator: websocket handshake failed: {err}");
                    return;
                }
            };

            let (mut write, mut read) = ws_stream.split();

            while let Some(message) = read.next().await {
                let message = match message {
                    Ok(msg) => msg,
                    Err(err) => {
                        eprintln!("Coordinator: websocket error: {err}");
                        break;
                    }
                };

                match message {
                    Message::Text(text) => {
                        let _incoming: Result<ToCoordinator, _> = serde_json::from_str(&text);
                        let response = FromCoordinator::Unknown;
                        let payload = match serde_json::to_string(&response) {
                            Ok(value) => value,
                            Err(err) => {
                                eprintln!("Coordinator: serialize error: {err}");
                                continue;
                            }
                        };

                        if let Err(err) = write.send(Message::Text(payload)).await {
                            eprintln!("Coordinator: send error: {err}");
                            break;
                        }
                    }
                    Message::Binary(_) => {}
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        });
    }
}
