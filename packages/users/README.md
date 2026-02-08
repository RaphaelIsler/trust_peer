# Users Package

Verwaltung von Benutzern mit automatischer Schlüsselgenerierung und Blockchain-Integration.

## Features

- SQLite-basierte Benutzerverwaltung
- Automatische Erstellung der `users.sqlite` Datenbank beim ersten Start
- Mehrere Benutzer pro Installation
- Keine Passwörter erforderlich (lokale Datenspeicherung)
- **Nutzt crypto-Package für Schlüsselgenerierung** (Ed25519)
- **Nutzt blockchain-Package für Blockchain-IDs**
- **Verwendet helper::UId für konsistente ID-Verwaltung**
- Zwei Blockchains pro Benutzer (öffentlich und privat)

## Abhängigkeiten

Das users-Package baut auf den bestehenden Packages auf:
- **crypto**: Für sichere Schlüsselgenerierung (`PrivateKey`, `PublicKey`)
- **blockchain**: Für Blockchain-Strukturen und IDs
- **helper**: Für konsistente UUID-Verwaltung (`helper::UId`)

## Benutzer-Daten

Jeder Benutzer hat folgende Eigenschaften:

- **Vorname** (erforderlich)
- **Nachname** (erforderlich)
- **Zweiter Vorname** (optional)
- **Geburtsdatum** (erforderlich)
- **Schlüsselpaar** (automatisch generiert)
- **Öffentliche Blockchain-ID**
- **Private Blockchain-ID**

## Verwendung

```rust
use users::{UserDatabase, Result};
use chrono::NaiveDate;

fn main() -> Result<()> {
    // Datenbank öffnen/erstellen
    let mut db = UserDatabase::open("users.sqlite")?;

    // Neuen Benutzer erstellen
    let user = db.create_user(
        "Max".to_string(),
        "Mustermann".to_string(),
        Some("Alexander".to_string()),
        NaiveDate::from_ymd_opt(1990, 5, 15).unwrap(),
    )?;

    println!("Benutzer erstellt: {} (ID: {})", user.full_name(), user.id);
    println!("Public Key: {}", user.public_key);
    println!("Öffentliche Blockchain: {}", user.public_blockchain_id);
    println!("Private Blockchain: {}", user.private_blockchain_id);

    // Alle Benutzer auflisten
    let users = db.list_users()?;
    for user in users {
        println!("- {}", user.full_name());
    }

    Ok(())
}
```

## Datenbank-Schema

Die `users.sqlite` Datenbank enthält eine Tabelle `users` mit folgenden Spalten:

- `id` (TEXT, PRIMARY KEY) - Eindeutige UUID
- `first_name` (TEXT) - Vorname
- `last_name` (TEXT) - Nachname
- `middle_name` (TEXT, NULL) - Zweiter Vorname
- `date_of_birth` (TEXT) - Geburtsdatum (ISO 8601)
- `public_key` (TEXT) - Öffentlicher Schlüssel (Hex)
- `private_key` (TEXT) - Privater Schlüssel (Hex)
- `public_blockchain_id` (TEXT) - ID der öffentlichen Blockchain
- `private_blockchain_id` (TEXT) - ID der privaten Blockchain
- `created_at` (TEXT) - Erstellungszeitpunkt (RFC 3339)

## Design-Entscheidungen

### Eine Datenbank für Benutzer

Die Benutzerdaten werden in einer separaten `users.sqlite` Datenbank gespeichert. Die Blockchains selbst werden separat verwaltet, die User-DB enthält nur die Blockchain-IDs als Referenzen. Dies bietet:

- Klare Trennung zwischen Benutzer- und Blockchain-Daten
- Einfache Sicherung von Benutzerdaten
- Bessere Performance bei Benutzer-Queries
- Flexibilität für zukünftige Erweiterungen

### Schlüsselspeicherung

Private Keys werden als Hex-Strings direkt in der Datenbank gespeichert. Da die Anwendung nur lokal läuft und keine Passwörter verwendet werden, ist dies akzeptabel. Für produktive Systeme sollte eine Verschlüsselung implementiert werden.
