# crypto

Kleines Rust-Package für Checksummen, Key-Management (Ed25519) und Signieren.

Funktionen:
- SHA3-256 Checksummen (hex)
- Erzeugen von Ed25519 Keypairs
- Speicherung der Keys in einer SQLite-Datenbank
- Signieren und Verifizieren von Daten

Features:
- default: `ed25519`
- `pq`: optional, verwendet `oqs` (post-quantum) wenn verfügbar

Beispiel:


## Algorithm Support

This crate currently supports Ed25519 (enabled by the default `ed25519` feature). Post-quantum (PQ) algorithms are planned via the optional `pq` feature which depends on the `oqs` crate and liboqs.

## Build & Test

Build the crate (default features - ed25519 enabled):

```bash
cd /home/snake/Projects/trust_peer
cargo build -p crypto
```

Run the tests (ed25519 tests run when the feature is enabled):

```bash
cargo test -p crypto
```

To build without ed25519 (no signing support):

```bash
cargo build -p crypto --no-default-features
```

To enable PQ feature (requires liboqs and system-level dependencies):

```bash
cargo build -p crypto --features pq
```