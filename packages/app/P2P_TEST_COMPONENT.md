# P2P Test Component

The `P2PTestComponent` is a Dioxus UI component for testing WebRTC peer-to-peer connections using the `p2p_webrtc` library.

## Features

- **Room-based Peer Discovery**: Join rooms via WebSocket signaling
- **Real-time Connection Status**: Visual indicators for connection state
- **Message Send/Receive**: Send and receive messages over WebRTC DataChannel
- **Error Handling**: Clear error messages for connection failures
- **Configuration Panel**: Set custom signaling server, peer ID, and room ID
- **Scrollable Message History**: View all sent/received messages

## Platform Support

- **Desktop** (Dioxus desktop feature)
- **Mobile** (Dioxus mobile feature)
- Not available on web

## Usage

### Import

```rust
use app::components::P2PTestComponent;
```

### Add to your Dioxus component

```rust
#[component]
fn MyPage() -> Element {
    rsx! {
        P2PTestComponent {}
    }
}
```

### Configuration

The component UI provides input fields for:

1. **Signaling Server** (default: `ws://localhost:3000`)
   - WebSocket URL for room-based peer discovery
   - Must be a valid WebSocket endpoint

2. **Peer ID** (default: `anonymous`)
   - Your unique identifier in the network
   - Can be any string (typically a username or UUID)

3. **Room ID** (required)
   - The room to join for peer discovery
   - Multiple peers in the same room will discover each other

## Connection Flow

1. **Idle**: Component starts in idle state
2. **Connecting**: User clicks "Connect" button
3. **Connected**: Successfully established P2P connection
4. **Failed**: Connection attempt failed (check error message)
5. **Disconnected**: User clicked "Disconnect" button

## Message Format

Messages are sent as UTF-8 bytes over the DataChannel. The component:
- Accepts any text message (max size depends on DataChannel buffer)
- Displays sent messages with `→` prefix
- Displays received messages with `←` prefix
- Logs all events to the console (use RUST_LOG=info to see logs)

## Example: Two Peers Connecting

### Setup Signaling Server

Start the example signaling server (from p2p_webrtc):

```bash
cargo run --example signaling_server -p p2p_webrtc
```

### Peer 1

1. Set Signaling Server to: `ws://localhost:3000`
2. Set Peer ID to: `peer1`
3. Set Room ID to: `test-room`
4. Click Connect

### Peer 2

On another machine/terminal:

1. Set Signaling Server to: `ws://localhost:3000` (or IP:3000 if remote)
2. Set Peer ID to: `peer2`
3. Set Room ID to: `test-room`
4. Click Connect

### Verify Connection

- Both peers should show "Connected" status
- Peer 1 can send messages → Peer 2 receives them
- Peer 2 can send messages → Peer 1 receives them

## Logging

The component uses the `log` crate for logging. To see detailed logs:

### Desktop

```bash
RUST_LOG=info dx serve
```

Or in code before running the app:

```rust
use log::Level;
env_logger::Builder::from_default_env()
    .filter_level(Level::Info)
    .init();
```

### Console Output

All logs are output to stderr. Look for messages like:

```
[P2P] Attempting connection to room: test-room
[P2P] Connected to room: test-room (Peer: peer1)
[P2P] Sent message: Hello, Peer2!
[P2P] Received message: Hi Peer1!
```

## Component State

The component manages:

- `room_id`: Current room ID
- `message_input`: Text being typed
- `messages`: History of sent/received messages
- `connection_status`: Current connection state
- `error_message`: Any error that occurred
- `peer_id`: Your unique peer identifier
- `signaling_server`: WebSocket server URL
- `p2p_instance`: Shared P2P handler

## Error Handling

Common errors:

1. **"Connection failed: Timeout"**
   - Signaling server not responding
   - Check server is running and URL is correct

2. **"Send failed: NotConnected"**
   - P2P connection dropped
   - Try disconnecting and reconnecting

3. **"Connection failed: Invalid room"**
   - Room ID format issue
   - Ensure no empty spaces

4. **"Connection failed: WebSocket error"**
   - Network connectivity issue
   - Check firewall and server status

## Advanced Usage

### Custom Signaling Server

If you need a custom signaling server, ensure it:
1. Speaks the WebSocket protocol
2. Implements the `SignalingMessage` protocol from `p2p_webrtc`
3. Routes peers by room ID
4. Relays SDP and ICE candidate messages

### ICE Servers

The component uses the p2p_webrtc default STUN servers. To customize, modify [p2p_test.rs](src/components/p2p_test.rs):

```rust
let config = P2pConfig::new(signaling_server, room)
    .with_timeout(30)
    .with_peer_id(peer_id)
    .with_stun_servers(vec!["stun:stun.example.com:3478".to_string()])
    .with_turn_servers(vec!["turn:turn.example.com".to_string()]);
```

## Testing

### Unit Test Example

```rust
#[test]
fn test_p2p_component_renders() {
    use dioxus::prelude::*;

    let mut vdom = VirtualDom::new(P2PTestComponent);
    vdom.rebuild();
    // Verify rendering
}
```

### Integration Test

1. Start signaling server
2. Run app with desktop feature: `cargo run -p app --features desktop`
3. Open two windows/instances
4. Test message exchange

## Performance Notes

- Message polling interval: 100ms
- Timeout: 30 seconds (configurable)
- DataChannel buffer: Configured by webrtc-rs (typically 16MB)
- Message size: Recommended <64KB for reliability

## Troubleshooting

### Component not showing

Ensure you're using a platform with desktop or mobile feature enabled:

```bash
cargo run -p app --features desktop
```

### No messages being received

1. Check both peers show "Connected" status
2. Verify signaling server logs show both peers joined
3. Look for console errors (RUST_LOG=debug)
4. Test with a simple ping/pong message first

### Connection hangs on "Connecting"

1. ICE candidate gathering may take time (up to 10 seconds)
2. If it takes >30 seconds, connection timeout occurs
3. Check firewall allows UDP on 10000-20000 range (WebRTC)

## Dependencies

The component requires:

```toml
dioxus = "0.7"
p2p_webrtc = { path = "../p2p_webrtc" }
log = "0.4"
tokio = { version = "1", features = ["full"] }
```

## File Location

[src/components/p2p_test.rs](src/components/p2p_test.rs) (~400 lines)

## Related

- [p2p_webrtc Library](../p2p_webrtc/)
- [p2p_webrtc Examples](../p2p_webrtc/examples/)
- [Dioxus Documentation](https://dioxuslabs.com)
