# Trust Peer - Benutzerverwaltung

## Übersicht

Die Benutzerverwaltung ist jetzt direkt im `app`-Package integriert und bietet folgende Features:

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
    pub created_at: DateTime<Utc>,       // Erstellungszeitpunkt
}
```

Zusätzliche Profildaten (z. B. `middle_name`, `date_of_birth`) werden pro Benutzer im Service gespeichert (Key-Value-DB), nicht in der zentralen User-Tabelle.

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
└── app/                      # Haupt-Anwendung
    ├── src/
    │   ├── components/
    │   │   └── user_manager.rs  # UI-Komponente für Benutzerverwaltung
    │   ├── user.rs          # User-Struct und Logik
    │   ├── user_db.rs       # SQLite-Datenbank-Verwaltung
    │   ├── user_databases.rs # Private/Public-DBs pro Benutzer
    │   └── error.rs         # Fehlertypen
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
    Ok(())
}
```

### 2. Benutzer erstellen (programmatisch)

```rust
use app::{UserDatabase, User};
use chrono::NaiveDate;

let db = UserDatabase::open("~/.trust_peer/data/users.sqlite").await?;

let user = db.create_user(
    "Max".to_string(),
    "Mustermann".to_string(),
    Some("Alexander".to_string()),
    NaiveDate::from_ymd_opt(1990, 5, 15).unwrap(),
)?;

println!("Benutzer erstellt: {}", user.full_name());
// Keys/Blockchains liegen in den User-Datenbanken (private/public)
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

- `Build: app (desktop)` - Baut die Desktop-App
- `Build: app (mobile)` - Baut die Mobile-App

## API-Dokumentation

Die Dokumentation zur Benutzerverwaltung liegt im app-Package.
