# P2P WebRTC Testing - Quick Start

This guide gets you testing peer-to-peer WebRTC connections in 5 minutes.

## Prerequisites

- Rust 1.70+ with Cargo
- Linux/macOS (desktop feature)
- Two terminals or two machines

## Step 1: Start the Signaling Server

Terminal 1:

```bash
cd /home/snake/Projects/trust_peer
cargo run --example signaling_server -p p2p_webrtc
```

Output should show:
```
Signaling server listening on 127.0.0.1:3000
```

## Step 2: Run the Desktop App

Terminal 2:

```bash
cd /home/snake/Projects/trust_peer
RUST_LOG=info cargo run -p app --features desktop
```

The app will compile and launch with logging enabled.

## Step 3: Connect First Peer

In the app window:

1. **Signaling Server**: Keep default `ws://localhost:3000`
2. **Peer ID**: Enter `peer1`
3. **Room ID**: Enter `test-room`
4. Click **🔗 Connect**

Wait for status to show: **Connected**

## Step 4: Connect Second Peer

Open another instance of the app:

```bash
RUST_LOG=info cargo run -p app --features desktop --bin desktop 2>/dev/null &
sleep 2
```

In the second app window:

1. **Signaling Server**: Keep default `ws://localhost:3000`
2. **Peer ID**: Enter `peer2`
3. **Room ID**: Enter `test-room`
4. Click **🔗 Connect**

Wait for status to show: **Connected**

## Step 5: Send a Message

In **Peer 1** window:

1. Message input should be enabled (status = Connected)
2. Type: `Hello from Peer 1!`
3. Click **📤 Send** or press Enter
4. Message appears in Peer 1's message list with `→` prefix

## Step 6: Verify Reception

In **Peer 2** window:

- Message should appear: `← Hello from Peer 1!`
- Click on Peer 2 message input
- Type: `Hi Peer 1!`
- Click **📤 Send**

In **Peer 1** window:

- Message should appear: `← Hi Peer 1!`

✅ **Congratulations! P2P connection working!**

## Console Logging

Watch the signaling in console:

```
[P2P] Attempting connection to room: test-room
[P2P] Connected to room: test-room (Peer: peer1)
[P2P] Received message: Hi Peer 1!
[P2P] Sent message: Hello from Peer 1!
```

## Troubleshooting

### "Connection failed" error

1. Check signaling server is running (Terminal 1)
2. Verify server shows both peers connecting:
   ```
   [INFO] Peer peer1 joined room: test-room
   [INFO] Peer peer2 joined room: test-room
   ```
3. Try different room ID (avoid special characters)

### No messages received

1. Verify both peers show **Connected** status
2. Check console for errors (RUST_LOG=debug for verbose output)
3. Try sending from the other peer

### App won't launch

1. Ensure `desktop` feature is specified: `--features desktop`
2. Check Rust version: `rustc --version` (should be 1.70+)
3. Try rebuilding: `cargo clean && cargo build -p app --features desktop`

## Next Steps

- [Read full P2P component docs](P2P_TEST_COMPONENT.md)
- [Explore p2p_webrtc library](../p2p_webrtc/)
- [Check p2p_demo example](../p2p_webrtc/examples/p2p_demo.rs)

## Architecture Overview

```
┌─────────────────────────────────────────┐
│  P2P Test Component (Dioxus UI)        │
│  ├── Connection Manager                 │
│  ├── Message Send/Receive               │
│  └── Status Display                     │
└────────────┬────────────────────────────┘
             │
             │ Uses
             ▼
┌─────────────────────────────────────────┐
│  p2p_webrtc Library (Rust)             │
│  ├── Signaling (WebSocket)              │
│  ├── PeerConnection (SDP/ICE)          │
│  └── DataChannel (Messages)             │
└────────────┬────────────────────────────┘
             │
             │ Connects via
             ▼
┌─────────────────────────────────────────┐
│  Signaling Server (WebSocket)           │
│  ├── Room Management                    │
│  ├── Peer Discovery                     │
│  └── Message Relay (SDP/ICE)           │
└─────────────────────────────────────────┘
```

## Feature Flags

For different platforms:

```bash
# Desktop (Linux/macOS)
cargo run -p app --features desktop

# Mobile (iOS/Android)
cargo run -p app --features mobile

# Server rendering
cargo build -p app --features server

# All features
cargo build -p app --all-features
```

## Performance Tips

1. **Local Testing**: Use `127.0.0.1:3000` for fastest responses
2. **Remote Testing**: Ensure UDP ports 10000-20000 are open
3. **Message Size**: Keep under 64KB for reliability
4. **Timeout**: Default 30 seconds, increase for slow networks

## File Structure

```
packages/
├── app/
│   ├── src/
│   │   └── components/
│   │       ├── mod.rs
│   │       └── p2p_test.rs (Component)
│   ├── P2P_TEST_COMPONENT.md (Full docs)
│   └── Cargo.toml
└── p2p_webrtc/
    ├── src/
    │   ├── lib.rs
    │   ├── api.rs (Main API)
    │   ├── signaling.rs (WebSocket)
    │   ├── peer.rs (WebRTC)
    │   └── data_channel.rs (Messages)
    └── examples/
        ├── signaling_server.rs
        └── p2p_demo.rs
```

## Useful Commands

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test -p p2p_webrtc

# Check for issues
cargo clippy -p app --features desktop

# View WebRTC logs
RUST_LOG=debug,webrtc=trace cargo run -p app --features desktop

# Kill lingering processes
pkill -f "cargo run" || true
```

## Advanced Testing

### Different Rooms (No connection)

Peer 1: Room ID = `test-room-1`
Peer 2: Room ID = `test-room-2`

Result: Both show Connected but don't exchange messages (different rooms)

### Network Emulation

Simulate latency:
```bash
sudo tc qdisc add dev lo root netem delay 100ms
# ... run tests ...
sudo tc qdisc delete dev lo root
```

### Multiple Peers

Start 3+ peers in same room - all will discover each other via signaling.

---

**Need help?** Check [P2P_TEST_COMPONENT.md](P2P_TEST_COMPONENT.md) for detailed documentation.
