# AGENTS_OWN

## Project Summary
- Rust workspace with multiple packages in packages/ (app, api, blockchain, coordinator, crypto, db, p2p_webrtc, weakup).
- UI uses Dioxus 0.7 (fullstack) in the app package.
- Build via cargo (workspace) and Dioxus CLI (dx serve/build).

## Architecture Notes
- User management lives inside packages/app (user.rs, user_db.rs, user_databases.rs, error.rs).
- Per-user data uses private/public SQLite databases in user-specific folders.
- Db entities implement db::DbEntity; migrations handled through db::DB.

## Conventions
- Dioxus 0.7 API: no cx/Scope/use_state; use signals/hooks.
- Prefer helper::Guid for GUID IDs when applicable.
- Avoid Rc/Arc when possible; use direct instances.

## Build & Run
- Desktop: cargo build -p app --features desktop --bin desktop
- Mobile: cargo build -p app --features mobile --bin mobile
- Web: cargo build -p app --features web --bin web
- Server: cargo build -p app --features server --bin server

## Notes
- Workspace root Cargo.toml defines members; avoid re-adding removed packages (e.g., users).