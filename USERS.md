# Trust Peer - Benutzerverwaltung

## Übersicht

Das `users`-Package bietet eine vollständige Benutzerverwaltung mit folgenden Features:

- ✅ SQLite-basierte lokale Datenspeicherung (`users.sqlite`)
- ✅ Automatische Datenbank-Initialisierung beim ersten Start
- ✅ Mehrere Benutzer pro Installation
- ✅ Keine Passwörter erforderlich (lokale Datenspeicherung)
- ✅ Automatische Ed25519-Schlüsselgenerierung pro Benutzer
- ✅ Zwei Blockchain-IDs pro Benutzer (öffentlich und privat)
- ✅ UI-Komponenten für Desktop und Mobile
- ✅ Android-Binary vorbereitet

## Benutzer-Struktur

Jeder Benutzer hat folgende Daten:

```rust
pub struct User {
    pub id: UserId,                      // Eindeutige UUID
    pub first_name: String,              // Vorname (Pflicht)
    pub last_name: String,               // Nachname (Pflicht)
    pub middle_name: Option<String>,     // Zweiter Vorname (optional)
    pub date_of_birth: NaiveDate,        // Geburtsdatum (Pflicht)
    pub public_key: String,              // Ed25519 Public Key (Hex)
    pub private_key: String,             // Ed25519 Private Key (Hex)
    pub public_blockchain_id: String,    // ID der öffentlichen Blockchain
    pub private_blockchain_id: String,   // ID der privaten Blockchain
    pub created_at: DateTime<Utc>,       // Erstellungszeitpunkt
}
```

## Datenspeicherung

### Desktop (Linux/macOS/Windows)
```
~/.trust_peer/data/users.sqlite
```

### Android
```
./data/users.sqlite
```
(Hinweis: Für Produktion sollte das Android-spezifische Datenverzeichnis über JNI abgerufen werden)

## Projekt-Struktur

```
packages/
├── users/                    # Benutzerverwaltungs-Package
│   ├── src/
│   │   ├── lib.rs           # Public API
│   │   ├── user.rs          # User-Struct und Logik
│   │   ├── user_db.rs       # SQLite-Datenbank-Verwaltung
│   │   └── error.rs         # Fehlertypen
│   └── Cargo.toml
│
└── app/                      # Haupt-Anwendung
    ├── src/
    │   ├── components/
    │   │   └── user_manager.rs  # UI-Komponente für Benutzerverwaltung
    │   ├── user_init.rs     # Initialisierungs-Logik
    │   └── bin/
    │       ├── desktop.rs   # Desktop-Binary
    │       └── mobile.rs    # Mobile/Android-Binary
    └── Cargo.toml
```

## Build-Befehle

### Desktop-App bauen
```bash
cargo build -p app --features desktop --bin desktop
```

### Desktop-App ausführen
```bash
cargo run -p app --features desktop --bin desktop
```

### Mobile-App bauen
```bash
cargo build -p app --features mobile --bin mobile
```

### Android-App bauen (mit dx)
```bash
dx build --platform android
```

### Nur users-Package bauen
```bash
cargo build -p users
```

### Tests ausführen
```bash
cargo test -p users
```

## Verwendung

### 1. Benutzerdatenbank initialisieren

Die Datenbank wird automatisch beim ersten Start der Desktop- oder Mobile-App initialisiert:

```rust
// In main()
if let Err(e) = init_database() {
    eprintln!("Fehler beim Initialisieren der Datenbank: {}", e);
}

fn init_database() -> anyhow::Result<()> {
    let db_path = app::user_init::init_user_database()?;
    app::user_init::ensure_test_user(&db_path)?;
    Ok(())
}
```

### 2. Benutzer erstellen (programmatisch)

```rust
use users::{UserDatabase, User};
use chrono::NaiveDate;

let mut db = UserDatabase::open("~/.trust_peer/data/users.sqlite")?;

let user = db.create_user(
    "Max".to_string(),
    "Mustermann".to_string(),
    Some("Alexander".to_string()),
    NaiveDate::from_ymd_opt(1990, 5, 15).unwrap(),
)?;

println!("Benutzer erstellt: {}", user.full_name());
println!("Public Key: {}", user.public_key);
```

### 3. UI-Komponente verwenden

Die `UserManager`-Komponente ist bereits in Desktop- und Mobile-Apps integriert:

```rust
use app::UserManager;

#[component]
fn App() -> Element {
    rsx! {
        UserManager {}
    }
}
```

## UI-Features

Die `UserManager`-Komponente bietet:

- 📋 Liste aller registrierten Benutzer
- ➕ Formular zum Erstellen neuer Benutzer
- 🔍 Detailansicht für jeden Benutzer (mit Keys und Blockchain-IDs)
- ⚠️ Fehlerbehandlung und Validierung
- 📱 Responsive Design für Desktop und Mobile

## Datenbank-Schema

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,              -- UUID als String
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    middle_name TEXT,                 -- Optional
    date_of_birth TEXT NOT NULL,      -- ISO 8601 Format
    public_key TEXT NOT NULL,         -- Hex-String
    private_key TEXT NOT NULL,        -- Hex-String
    public_blockchain_id TEXT NOT NULL,
    private_blockchain_id TEXT NOT NULL,
    created_at TEXT NOT NULL          -- RFC 3339 Format
);

CREATE INDEX idx_users_name ON users(last_name, first_name);
```

## Sicherheitshinweise

⚠️ **Wichtig für Produktion:**

1. Die Private Keys werden aktuell **unverschlüsselt** in der Datenbank gespeichert
2. Für produktive Systeme sollte eine Verschlüsselung implementiert werden
3. Die Datenbank-Datei sollte mit entsprechenden Dateiberechtigungen geschützt werden
4. Für Android sollte das App-spezifische Datenverzeichnis über JNI abgerufen werden

## Nächste Schritte

- [ ] Private Key-Verschlüsselung implementieren
- [ ] Blockchain-Integration (tatsächliche Erstellung der Blockchains)
- [ ] Benutzer-Authentifizierung für Blockchain-Zugriff
- [ ] Export/Import von Benutzerdaten
- [ ] Backup-Funktionalität
- [ ] Android-spezifisches Datenverzeichnis über JNI
- [ ] Benutzer-Avatar-Unterstützung
- [ ] Erweiterte Benutzerprofile

## Entwicklung

### VSCode Tasks

Die folgenden Tasks sind in `.vscode/tasks.json` verfügbar:

- `Build: users` - Baut nur das users-Package
- `Build: app (desktop)` - Baut die Desktop-App
- `Build: app (mobile)` - Baut die Mobile-App
- `Test: users` - Führt Tests für das users-Package aus

## API-Dokumentation

Ausführliche API-Dokumentation ist im [users/README.md](packages/users/README.md) verfügbar.
