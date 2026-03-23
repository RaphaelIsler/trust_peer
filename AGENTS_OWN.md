# AGENTS_OWN

## Project Summary
- Rust workspace with multiple packages in packages/ (app, api, blockchain, coordinator, crypto, db, p2p_webrtc, weakup).
- UI uses Dioxus 0.7 (fullstack) in the app package.
- Build via cargo (workspace) and Dioxus CLI (dx serve/build).

## Architecture — LedgerNode (current, as of 2026-03)

### Core concept
- There is NO separate User entity. One LedgerNode = one identity.
- A LedgerNode can have multiple `Identification` values (enum, currently `NameAndBirth`).
- LedgerNodes are discovered at startup by scanning the base data dir for subdirs that contain `private.sqlite`. The folder name IS the private chain UUID.

### LedgerNode data layout (per node, under `~/.trust_peer/data/<private-chain-uuid>/`)
- `private.sqlite` — private blockchain (BlockEntry values)
- `public.sqlite` — public blockchain (verification entries + identification hashes)
- `key.sqlite` — key pairs (crypto::KeyMeta), key-value store (KeyValue), peer connections

### Identification storage
- Identifications are stored in `key.sqlite` via `KeyValue` (key: `identification.0`, `.1`, …; count: `identification.count`).
- On creation and when adding a new Identification, a SHA-256 hash (`crypto::Hash`) of the JSON-serialized Identification is written as `BlockEntry::Identification { data }` to the **public chain** as an integrity proof.
- For hashing always use `crypto::Hash::new(bytes)` from the `crypto` crate — never raw `sha2`.

### Interface layers (Frontend ↔ Backend)
- `ledger_node::identification::Identification` — the shared type; also exposes Dioxus components (`Create`, `Show`).
- `ledger_node::interface::{ToFrontend, ToBackend}` — per-node messages.
- `app::interface::{ToFrontend, ToBackend}` — top-level router:
  - `ToBackend::Init` — load all nodes
  - `ToBackend::Create(Identification)` — create a new LedgerNode
  - `ToBackend::Ledger { private, msg }` — routed to specific node
  - `ToFrontend::Init(Vec<(BlockchainId, Money)>)` — list of all nodes
  - `ToFrontend::Ledger { private, msg }` — response from a specific node
- `ledger_node::frontend::Frontend` — client-side state per node (private/public ids, current money, identifications, `display_name()`).

### Backend service (app::backend::Service)
- Owns a `HashMap<String, LedgerService>` keyed by private chain id string.
- Discovers existing nodes via `discover_ledger_nodes()` (UUID dirs with `private.sqlite`).
- `LedgerService::create_new_instance(base_path, &new_uuid_id, identification)` for new nodes.
- `LedgerService::open(base_path, &id)` for existing nodes.
- Async events forwarded from each node to the frontend via an event channel.

### Frontend (app::frontend)
- Single `HashMap<String, ledger_node::frontend::Frontend>` signal, no user_store.
- Empty state → shows `identification::Create` form → sends `ToBackend::Create(ident)`.
- Non-empty → renders `ledger_node::frontend::Overview` per node.

### Planned (manager/ in user/ module)
- `user/manager/user_card.rs` and `create_user_form.rs` contain detailed UI for blockchain/connection management meant to be reintegrated into LedgerNode with proper Frontend→Backend architecture over `ledger_node::interface`.

## Conventions
- Dioxus 0.7 API: no cx/Scope/use_state; use signals/hooks.
- `helper::UId<T>::parse_str(s)` — NOT `from_str`. `UId` does not impl `std::str::FromStr`.
- Use `crypto::Hash::new(bytes)` for SHA-256; never use `sha2` directly in app code.
- Prefer module namespaces (`mod::Type`) over globally prefixed names.
- Keep low-complexity types in one file; split into module folder when growing.
- `#[derive(helper::ServiceWrapper)]` on a message enum generates async wrapper methods (snake_case) on the Service type.
- For `helper::ServiceWrapper`, each message variant must have one `oneshot::Sender<...>` field named `tx`.

## Build & Run
- Desktop: `cargo build -p app --features desktop --bin desktop`
- Mobile: `cargo build -p app --features mobile --bin mobile`
- Web: `cargo build -p app --features web --bin web`
- Server: `cargo build -p app --features server --bin server`
- Quick check: `cargo build -p app --features desktop --bin desktop`

## Notes
- Workspace root Cargo.toml defines members; avoid re-adding removed packages.
- `sha2` is no longer a direct dependency of `app`; it is replaced by `crypto::Hash`.
- The `packages/app/src/user/` directory still exists as archive for the manager UI; it is not compiled into the main flow (`pub mod user` is commented out in lib.rs).