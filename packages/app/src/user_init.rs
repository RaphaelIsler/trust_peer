use anyhow::Result;
use chrono::NaiveDate;
use users::{UserDatabase, User};
use std::path::PathBuf;

/// Initialisiert die Benutzerdatenbank und gibt den Pfad zurück
pub async fn init_user_database() -> Result<PathBuf> {
    // Bestimme Speicherort für die Datenbank
    let data_dir = get_data_directory()?;
    let db_path = data_dir.join("users.sqlite");

    // Erstelle das Verzeichnis, falls es nicht existiert
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Öffne/erstelle die Datenbank (initialisiert automatisch)
    let _db = UserDatabase::open(&db_path).await?;

    println!("Benutzerdatenbank initialisiert: {}", db_path.display());

    Ok(db_path)
}

/// Gibt das Datenverzeichnis für die Anwendung zurück
fn get_data_directory() -> Result<PathBuf> {
    // Plattform-spezifische Datenverzeichnisse
    #[cfg(target_os = "android")]
    {
        // Für Android: verwende das App-spezifische Datenverzeichnis
        // Dies müsste über JNI oder ähnliches vom Android-System abgerufen werden
        // Fallback auf ein relatives Verzeichnis für die Entwicklung
        Ok(PathBuf::from("./data"))
    }

    #[cfg(not(target_os = "android"))]
    {
        // Für Desktop: verwende das Benutzerverzeichnis
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))?;
        let data_dir = PathBuf::from(home)
            .join(".trust_peer")
            .join("data");
        Ok(data_dir)
    }
}

/// Gibt den Pfad zur Benutzerdatenbank zurück
pub fn get_db_path() -> Result<PathBuf> {
    let data_dir = get_data_directory()?;
    Ok(data_dir.join("users.sqlite"))
}

/// Beispiel: Erstellt einen Test-Benutzer, wenn noch keine Benutzer existieren
pub async fn ensure_test_user(db_path: &PathBuf) -> Result<()> {
    let db = UserDatabase::open(db_path).await?;

    let user_count = db.count_users().await?;

    if user_count == 0 {
        println!("Keine Benutzer gefunden. Erstelle Test-Benutzer...");

        let (user, _user_dbs) = db.create_user(
            "Max".to_string(),
            "Mustermann".to_string(),
            Some("Alexander".to_string()),
            NaiveDate::from_ymd_opt(1990, 5, 15)
                .ok_or_else(|| anyhow::anyhow!("Ungültiges Datum"))?,
        ).await?;

        println!("Test-Benutzer erstellt:");
        println!("  Name: {}", user.full_name());
        println!("  ID: {:?}", user.id);
    } else {
        println!("Gefundene Benutzer: {}", user_count);
    }

    Ok(())
}

/// Listet alle Benutzer auf
pub async fn list_all_users(db_path: &PathBuf) -> Result<Vec<User>> {
    let db = UserDatabase::open(db_path).await?;
    let users = db.list_users().await?;
    Ok(users)
}
