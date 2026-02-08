use anyhow::Result;
use chrono::NaiveDate;
use users::{UserDatabase, User};
use std::path::PathBuf;

/// Initializes the user database and returns the path
pub async fn init_user_database() -> Result<PathBuf> {
    // Determine storage location for the database
    let data_dir = get_data_directory()?;
    let db_path = data_dir.join("users.sqlite");

    // Create the directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open/create the database (initializes automatically)
    let _db = UserDatabase::open(&db_path).await?;

    println!("User database initialized: {}", db_path.display());

    Ok(db_path)
}

/// Returns the data directory for the application
fn get_data_directory() -> Result<PathBuf> {
    // Platform-specific data directories
    #[cfg(target_os = "android")]
    {
        // For Android: use the app-specific data directory
        // This would need to be retrieved from the Android system via JNI or similar
        // Fallback to a relative directory for development
        Ok(PathBuf::from("./data"))
    }

    #[cfg(not(target_os = "android"))]
    {
        // For Desktop: use the user directory
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))?;
        let data_dir = PathBuf::from(home)
            .join(".trust_peer")
            .join("data");
        Ok(data_dir)
    }
}

/// Returns the path to the user database
pub fn get_db_path() -> Result<PathBuf> {
    let data_dir = get_data_directory()?;
    Ok(data_dir.join("users.sqlite"))
}

/// Example: Creates a test user if no users exist yet
pub async fn ensure_test_user(db_path: &PathBuf) -> Result<()> {
    let db = UserDatabase::open(db_path).await?;

    let user_count = db.count_users().await?;

    if user_count == 0 {
        println!("No users found. Creating test user...");

        let (user, _user_dbs) = db.create_user(
            "Max".to_string(),
            "Mustermann".to_string(),
            Some("Alexander".to_string()),
            NaiveDate::from_ymd_opt(1990, 5, 15)
                .ok_or_else(|| anyhow::anyhow!("Invalid date"))?,
        ).await?;

        println!("Test user created:");
        println!("  Name: {}", user.full_name());
        println!("  ID: {:?}", user.id);
    } else {
        println!("Found users: {}", user_count);
    }

    Ok(())
}

/// Lists all users
pub async fn list_all_users(db_path: &PathBuf) -> Result<Vec<User>> {
    let db = UserDatabase::open(db_path).await?;
    let users = db.list_users().await?;
    Ok(users)
}
