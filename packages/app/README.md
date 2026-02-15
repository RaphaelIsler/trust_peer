# App

Unified Dioxus Fullstack application with UI components and server functions.

## Features

- **Fullstack Architecture**: Server rendering + client interactivity
- **User Management**: User database and authentication (Desktop/Mobile)
- **P2P WebRTC Testing**: Test peer-to-peer connections in real-time (Desktop/Mobile)
- **Real-time Messaging**: Send and receive messages over WebRTC DataChannel
- **Multiple Platforms**: Web, Desktop, and Mobile support

## Building

- **Server**: `cargo build --bin server --features server`
- **Web**: `cargo build --bin web --features web`
- **Desktop**: `cargo build --bin desktop --features desktop`
- **Mobile**: `cargo build --bin mobile --features mobile`

## Running

- **Server**: `dx serve --platform fullstack`
- **Web**: `dx serve --platform web`
- **Desktop**: `dx serve --platform desktop`
- **Mobile**: `dx serve --platform mobile`

## Components

### P2PTestComponent

A full-featured UI component for testing WebRTC peer-to-peer connections.

**Quick Start:**

```bash
# Terminal 1: Start signaling server
cargo run --example signaling_server -p p2p_webrtc

# Terminal 2: Run app with desktop
RUST_LOG=info cargo run -p app --features desktop
```

See [P2P_QUICKSTART.md](P2P_QUICKSTART.md) for detailed instructions.

**Usage:**

```rust
use app::components::P2PTestComponent;

#[component]
fn MyPage() -> Element {
    rsx! {
        P2PTestComponent {}
    }
}
```

### UserManager Component

Manage users in the database (Desktop/Mobile only).

## Documentation

- [P2P_QUICKSTART.md](P2P_QUICKSTART.md) - 5-minute quick start guide
- [P2P_TEST_COMPONENT.md](P2P_TEST_COMPONENT.md) - Complete component documentation
- [P2P_USAGE_EXAMPLES.md](P2P_USAGE_EXAMPLES.md) - Integration examples
- [P2P_COMPONENT_SUMMARY.md](P2P_COMPONENT_SUMMARY.md) - Implementation details

## Directory Structure

```
src/
├── lib.rs              # Root library exports
├── server_fn.rs        # Server functions (fullstack)
├── user_init.rs        # User initialization
├── components/         # Reusable UI components
│   ├── mod.rs
│   ├── hero.rs         # Hero component
│   ├── echo.rs         # Echo component
│   ├── user_manager.rs # User management (Desktop/Mobile)
│   └── p2p_test.rs     # P2P testing (Desktop/Mobile)
└── bin/                # Platform-specific entries
    ├── web.rs
    ├── desktop.rs
    ├── mobile.rs
    └── server.rs
```

## Dependencies

- **dioxus**: UI framework with fullstack support
- **p2p_webrtc**: Peer-to-peer WebRTC library (Desktop/Mobile only)
- **users**: User management library
- **tokio**: Async runtime
- **log**: Structured logging
- **serde**: Serialization
- **anyhow**: Error handling

## Platform Support

| Feature | Web | Desktop | Mobile | Server |
|---------|-----|---------|--------|--------|
| P2P Testing | ❌ | ✅ | ✅ | ❌ |
| User Management | ❌ | ✅ | ✅ | ✅ |
| Fullstack Rendering | ✅ | ✅ | ✅ | ✅ |

## Development

### Setup

1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Install Dioxus CLI: `curl --proto '=https' --tlsv1.2 -sSf https://dioxus.dev/install.sh | sh`
3. Clone repository: `git clone <repo-url>`
4. Navigate: `cd packages/app`

### Development Server

```bash
# Desktop with logging
RUST_LOG=debug dx serve --platform desktop

# Web development
dx serve --platform web

# With hot reload
cargo watch -x "dx serve --platform desktop"
```

### Testing

```bash
# Run all tests
cargo test -p app

# Test specific component
cargo test -p app --test p2p_test

# With logging
RUST_LOG=debug cargo test -p app -- --nocapture
```

### Linting

```bash
# Check code
cargo clippy -p app --all-features

# Format code
cargo fmt -p app

# Fix issues
cargo fix -p app --allow-dirty
```

## Environment Variables

- `RUST_LOG`: Set logging level (`debug`, `info`, `warn`, `error`)
  ```bash
  RUST_LOG=info cargo run -p app --features desktop
  ```

- `RUST_BACKTRACE`: Enable backtrace on panic
  ```bash
  RUST_BACKTRACE=1 cargo run -p app --features desktop
  ```

## Performance

- **Component Load Time**: < 100ms
- **P2P Connection**: 2-10 seconds (includes ICE gathering)
- **Message Latency**: < 100ms (local network)
- **Memory**: ~50MB (typical usage)

## Troubleshooting

### Component not showing

Ensure the feature is enabled:

```bash
# ✅ Correct
cargo run -p app --features desktop

# ❌ Wrong
cargo run -p app  # Missing --features
```

### P2P connection fails

1. Check signaling server is running: `cargo run --example signaling_server -p p2p_webrtc`
2. Verify server URL: `ws://localhost:3000` (or your custom URL)
3. Check console logs: `RUST_LOG=debug`
4. Ensure both peers in same room

### Build takes too long

1. Use release mode: `cargo build -p app --features desktop --release`
2. Enable incremental compilation: `CARGO_INCREMENTAL=1`
3. Use mold linker: `RUSTFLAGS="-C link-arg=-fuse-ld=mold"`

## Contributing

1. Follow Rust style guide: `cargo fmt`
2. Run clippy: `cargo clippy --all-features`
3. Add tests for new features
4. Update documentation
5. Create pull request

## License

[Your License Here]

## Resources

- [Dioxus Documentation](https://dioxuslabs.com)
- [p2p_webrtc Library](../p2p_webrtc/)
- [WebRTC Spec](https://w3c.github.io/webrtc-pc/)
- [Tokio Runtime](https://tokio.rs)

---

**Last Updated**: 2024
