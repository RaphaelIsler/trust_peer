# CLAUDE.md — Trust Peer

## Project Overview

Decentralized peer-to-peer trust ledger system with identity-based blockchains. Each identity manages two separate blockchains (private + public), money accounts, and peer connections via WebRTC.

## Language

**All code, comments, variable names, commit messages, and documentation must be written in English.** No German in source files.

## Workspace Structure

```
packages/
  app/            # Dioxus fullstack app (Desktop/Mobile/Web/Server)
  api/            # Shared protocol types and server configuration
  blockchain/     # Generic blockchain implementation
  core_types/     # Primitive types (Timestamp, KeyValue, Q32_32)
  crypto/         # Cryptography (Ed25519, SHA-256, sealed_box)
  coordinator/    # WebSocket coordinator server
  db/             # SQLite abstraction with DbEntity trait
  p2p_webrtc/     # WebRTC P2P networking library
  weakup/         # Push notification service (FCM + APNS)
```

## Architecture

### Layers

```
Dioxus Frontend (Signals)
        ↓ ToBackend messages
app::backend::Service  (HashMap<Id, LedgerService>)
        ↓
ledger_node::backend::Service
        ↓
Blockchain<BlockEntry> (private + public)
        ↓
SQLite via DbEntity trait
```

### Service Pattern

All backend services communicate via `tokio::sync::mpsc` channels. Single-reply interactions use `oneshot::Sender`. The `helper::ServiceWrapper` derive macro auto-generates type-safe wrapper methods:

```rust
#[derive(helper::ServiceWrapper)]
pub enum Msg {
    GetPublicPrivate(oneshot::Sender<Result<...>>),
    FromGui(ToBackend, oneshot::Sender<...>),
}
```

### Blockchain Model

`Blockchain<T>` is generic over the entry type. Each identity has:
- `private.sqlite` → `Blockchain<BlockEntry>` (transactions)
- `public.sqlite` → `Blockchain<BlockEntry>` (verifications, identity hashes)

### Platform Support (Feature Gates)

```toml
desktop  = ["dioxus/desktop", "backend", "frontend"]
mobile   = ["dioxus/mobile", "backend", "frontend"]
web      = ["dioxus/web"]
server   = ["dioxus/server", "tokio", ...]
backend  = ["tokio", "p2p_webrtc", "db", "blockchain", "crypto", ...]
```

## Key Types

| Type | Package | Purpose |
|------|---------|---------|
| `Blockchain<T>` | `blockchain` | Generic blockchain |
| `Block<T>` | `blockchain` | Single block with header |
| `BlockEntry` | `app` | Enum: Verification / Identification / Link / CurrentAmount |
| `DbEntity` | `db` | Trait for SQLite persistence |
| `Hash` | `crypto` | SHA-256 (32 bytes) |
| `PrivateKey` / `PublicKey` | `crypto` | Ed25519 key pair |
| `Signature` | `crypto` | Ed25519 signature |
| `KeyValue` | `core_types` | Typed key-value pairs |
| `Timestamp` | `core_types` | UNIX timestamp with arithmetic |
| `Money` | `app` | Monetary value type |
| `Identification` | `app/ledger_node` | Identity proof (NameAndBirth) |
| `BankAccount` | `app` | Money account abstraction |
| `P2pWebRtc` | `p2p_webrtc` | High-level WebRTC API |
| `RoomId` / `PeerId` | `p2p_webrtc` | Strongly typed WebRTC IDs |

## DbEntity Trait

All persisted types implement this trait:

```rust
pub trait DbEntity {
    type Id;
    fn table_name() -> &'static str;
    fn schema_version() -> u32;
    async fn create_table(conn: &SqlitePool) -> Result<()>;
    async fn update_table(conn: &SqlitePool, from: u32, to: u32) -> Result<()>;
    async fn write(&self, conn: &SqlitePool) -> Result<()>;
    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>>;
    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()>;
    async fn list(conn: &SqlitePool) -> Result<Vec<Self>>;
}
```

Schema migrations are versioned via the `schema_migrations` table.

## Coding Conventions

### Error Handling
- `anyhow::Result<T>` as the default return type
- Custom error enums with `thiserror::Error` where needed (e.g. `p2p_webrtc::error`)
- No `.unwrap()` outside of tests

### Async
- `tokio::spawn` for independent tasks
- `mpsc` for persistent channels, `oneshot` for request-response
- `#[tokio::main]` for binaries

### IDs
- Use strongly typed IDs: `helper::I64Id<Marker>` (incremental) and `helper::UId<Marker>` (UUID)
- Never use raw `i64` or `String` as IDs

### Cryptography
- Always use `crypto::Hash::new(bytes)`, never `sha2` directly
- Respect `#[cfg(feature = "ed25519")]` feature guards
- Private keys never leave their service

### Database
- Always pass `&SqlitePool`, never individual connections
- Store complex types as JSON in SQLite (via `serde_json` / `bincode`)

### Dioxus UI
- `#[component]` for function components
- `rsx!` macro for element trees
- Signals for reactive state
- View types use `_view` suffix (e.g. `BlockHeaderView`)
- **Styling: use CSS classes, not inline `style="..."`**. All styles go in `packages/app/assets/main.css`. Use BEM-style class names (e.g. `ledger-node__header`, `btn--primary`). Inline styles are only acceptable for truly dynamic values (e.g. a computed pixel width).
- **Whenever a new component or UI element is added or modified, its CSS classes must be defined or updated in `packages/app/assets/main.css` in the same step.** Never leave a class referenced in `rsx!` without a corresponding rule in the stylesheet.

### Multi-Language (i18n)
- **Every user-facing string in a frontend component must go through `i18n.t(Key::...)`**. Never hardcode display strings directly in `rsx!`.
- Add new keys to both `en()` and `de()` in `packages/app/src/i18n.rs` whenever a new UI string is introduced.
- The OS locale is detected automatically at startup (`LANG` / `LANGUAGE` / `LC_ALL`). Users can override via the `LanguageSelector` component.
- Usage pattern in any component:
  ```rust
  let i18n = use_i18n();
  // ...
  rsx! { span { "{i18n.t(Key::MyKey)}" } }
  ```

### Message Types
- `ToBackend` / `FromBackend` at app level
- `ToFrontend` / `ToCoordinator` / `FromCoordinator` for external communication
- JSON across all network boundaries

## Build Commands

```bash
# Desktop
cargo build -p app --features desktop --bin desktop

# Web
cargo build -p app --features web --bin web

# Server (fullstack backend)
cargo build -p app --features server --bin server

# P2P signaling server
cargo build -p p2p_webrtc --bin signaling_server

# Push notification service
cargo build -p weakup
```

## Key Dependencies

| Crate | Usage |
|-------|-------|
| `dioxus` 0.7 | Reactive UI framework (Desktop/Mobile/Web) |
| `tokio` | Async runtime |
| `sqlx` 0.8 | SQLite with compile-time checked queries |
| `serde` / `serde_json` | Serialization |
| `ed25519-dalek` | Signatures (optional, feature `ed25519`) |
| `webrtc` | WebRTC peer connections |
| `tokio-tungstenite` | WebSocket client/server |
| `axum` | HTTP router (weakup service) |
| `jsonwebtoken` | JWT signing for FCM/APNS |
| `thiserror` | Declarative error types |
| `helper` (git) | UUID/ID types and ServiceWrapper macro |
| `oqs` | Post-quantum cryptography (optional) |
