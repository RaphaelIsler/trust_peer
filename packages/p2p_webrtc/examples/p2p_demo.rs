//! Example: Two peers connecting via WebSocket signaling
//!
//! This example demonstrates:
//! - Connecting to a signaling server
//! - Establishing a P2P connection via WebRTC
//! - Exchanging messages over DataChannel
//! - Handling connection events
//!
//! Usage:
//!   cargo run --example p2p_demo -- [signaling_server_url] [room_id]
//!
//! Example:
//!   cargo run --example p2p_demo -- ws://localhost:3000 test-room

use p2p_webrtc::{P2pConfig, P2pWebRtc, init_logger};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    let args: Vec<String> = std::env::args().collect();
    let signaling_server = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "ws://localhost:3000".to_string());
    let room_id = args.get(2).cloned().unwrap_or_else(|| "test-room".to_string());

    println!("Starting P2P connection example");
    println!("  Signaling server: {}", signaling_server);
    println!("  Room ID: {}", room_id);

    // Create configuration
    let config = P2pConfig::new(signaling_server, room_id)
        .with_timeout(30)
        .with_stun_servers(vec![]);

    // Create P2P handler
    let mut p2p = P2pWebRtc::new(config);

    println!("Connecting to peer...");
    match p2p.connect().await {
        Ok(_) => {
            println!("✓ Connected to peer!");

            // Send a message
            println!("Sending message...");
            let message = b"Hello from peer!";
            p2p.send(message).await?;
            println!("✓ Message sent: {:?}", String::from_utf8_lossy(message));

            // Try to receive messages for 10 seconds
            println!("Waiting for incoming messages (10 seconds)...");
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_secs(10) {
                if let Ok(Some(data)) = p2p.try_recv().await {
                    println!(
                        "✓ Received message: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
                sleep(Duration::from_millis(100)).await;
            }

            // Close connection
            println!("Closing connection...");
            p2p.close().await?;
            println!("✓ Connection closed");
        }
        Err(e) => {
            eprintln!("✗ Connection failed: {}", e);
            eprintln!("Make sure the signaling server is running at: {}",
                p2p.config.signaling_server);
        }
    }

    Ok(())
}
