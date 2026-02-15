# P2P WebRTC Library

A modern, async-first Rust library for establishing WebRTC peer-to-peer connections in mobile and desktop applications.

## Features

- **WebSocket Signaling**: Room-based signaling for seamless peer discovery
- **Full WebRTC Support**: SDP offer/answer exchange with ICE candidate handling
- **Reliable DataChannel**: Automatic creation and management of ordered, reliable data channels
- **STUN/TURN Support**: Built-in support for NAT traversal via STUN and TURN servers
- **Async API**: Fully async API using Tokio for non-blocking operations
- **Mobile Ready**: Designed for iOS and Android integration via FFI
- **Production Logging**: Comprehensive logging for debugging P2P connectivity
- **Modular Architecture**: Clean separation of concerns for easy maintenance and extension

## Quick Start

### Add to Cargo.toml

```toml
[dependencies]
p2p_webrtc = { path = "packages/p2p_webrtc" }
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use p2p_webrtc::{P2pConfig, P2pWebRtc, init_logger};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    // Create configuration
    let config = P2pConfig::new(
        "ws://localhost:3000".to_string(),
        "my-room".to_string(),
    )
    .with_timeout(30);

    // Create and connect
    let mut p2p = P2pWebRtc::new(config);
    p2p.connect().await?;

    // Send data
    p2p.send(b"Hello, peer!").await?;

    // Receive data
    while let Ok(Some(data)) = p2p.try_recv().await {
        println!("Received: {:?}", String::from_utf8_lossy(&data));
    }

    p2p.close().await?;
    Ok(())
}
```

## API Reference

### `P2pConfig`

Configuration builder for P2P connections:

```rust
let config = P2pConfig::new(signaling_server, room_id)
    .with_timeout(30)
    .with_stun_servers(vec![])
    .with_turn_server(
        vec!["turn:example.com".to_string()],
        "username".to_string(),
        "credential".to_string(),
    )
    .with_peer_id("my-peer-id".to_string());
```

### `P2pWebRtc`

Main connection handler:

```rust
// Create handler
let mut p2p = P2pWebRtc::new(config);

// Connect to a peer
p2p.connect().await?;

// Send data (async)
p2p.send(b"data").await?;

// Receive data (non-blocking)
if let Some(data) = p2p.try_recv().await? {
    println!("Got: {:?}", data);
}

// Receive data (blocking)
let data = p2p.recv().await?;

// Close connection
p2p.close().await?;
```

## Architecture

### Modules

- **`signaling`**: WebSocket client for room-based signaling
- **`peer`**: WebRTC PeerConnection wrapper with SDP and ICE handling
- **`data_channel`**: DataChannel management for reliable message delivery
- **`api`**: High-level async API for easy integration

### Message Flow

```
┌─────────────────────────────────────────────────────┐
│         Signaling Server (WebSocket)                │
└────────┬───────────────────────────────────┬────────┘
         │                                   │
    ┌────▼────┐                       ┌─────▼───┐
    │ Peer A  │ ◄─── SDP/ICE ───────► │ Peer B  │
    │         │                       │         │
    │ (Init)  │                       │ (Resp)  │
    └─────────┘                       └─────────┘
         │                                   │
         │  ◄──── WebRTC DataChannel ──────► │
         │      (Reliable, Ordered)          │
         └───────────────────────────────────┘
```

## Running the Examples

### Start the Signaling Server

```bash
cargo run --example signaling_server -- 3000
```

This starts a WebSocket signaling server on `ws://localhost:3000`.

### Run the P2P Demo

In separate terminals:

```bash
# Terminal 1 (Peer A)
cargo run --example p2p_demo -- ws://localhost:3000 test-room

# Terminal 2 (Peer B)
cargo run --example p2p_demo -- ws://localhost:3000 test-room
```

Both peers will:
1. Connect to the signaling server
2. Discover each other in the room
3. Establish a P2P connection
4. Exchange WebRTC messages over the DataChannel
5. Close the connection gracefully

## NAT Traversal

The library supports both STUN and TURN servers for NAT traversal:

### STUN (Session Traversal Utilities for NAT)

Used for peer discovery and public IP address detection:

```rust
config.with_stun_servers(vec![

])
```

### TURN (Traversal Using Relays around NAT)

Used as a fallback when direct P2P is not possible:

```rust
config.with_turn_server(
    vec!["turn:turnserver.example.com:3478".to_string()],
    "username".to_string(),
    "credential".to_string(),
)
```

## Logging and Debugging

Enable debug logging to troubleshoot connection issues:

```rust
// Initialize logger with debug level
init_logger();

// Set RUST_LOG environment variable
// export RUST_LOG=debug
// cargo run --example p2p_demo
```

Output includes:
- Signaling events (join, offer, answer, ICE candidates)
- PeerConnection state transitions
- DataChannel lifecycle events
- Error messages with full context

## Integration with Mobile Apps

### iOS Integration

1. Build the library for iOS:
   ```bash
   cargo build --target aarch64-apple-ios
   ```

2. Create Swift FFI bindings:
   ```swift
   // Import the Rust library
   import p2p_webrtc

   // Use the exported C API
   let p2p = P2pWebRtc(config)
   p2p.connect()
   ```

### Android Integration

1. Build for Android:
   ```bash
   cargo build --target aarch64-linux-android
   ```

2. Create JNI bindings in Kotlin:
   ```kotlin
   external fun p2pConnect(config: String): Int
   external fun p2pSend(data: ByteArray)
   external fun p2pRecv(): ByteArray?
   ```

## Dependencies

- **tokio**: Async runtime
- **tokio-tungstenite**: WebSocket client
- **webrtc**: WebRTC implementation
- **serde**: Serialization
- **log**: Logging facade
- **anyhow**: Error handling

## Performance Considerations

- DataChannel uses ordered and reliable settings for guaranteed delivery
- Message size: Recommend keeping messages under 16 KB
- Connection timeout: Default 30 seconds, adjustable via config
- ICE gathering: Can take 1-5 seconds depending on network
- TURN fallback: Adds 100-200ms latency compared to direct P2P

## Troubleshooting

### Connection Timeouts

1. Check signaling server is running and accessible
2. Verify STUN server is reachable (test with a public STUN server)
3. Increase timeout: `config.with_timeout(60)`

### DataChannel Not Opening

1. Ensure both peers reach the `Connected` state
2. Check browser console for ICE candidate errors
3. Verify firewall allows UDP/TCP for WebRTC

### No Messages Received

1. Confirm DataChannel is open before sending
2. Check that receiver is calling `recv()` or `try_recv()`
3. Enable debug logging to see message flow

## Contributing

Contributions are welcome! Please ensure:
- Code follows Rust conventions
- All tests pass: `cargo test --workspace`
- No warnings: `cargo clippy --workspace`
- Documentation is updated

## License

Apache 2.0
