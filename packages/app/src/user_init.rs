use anyhow::Result;
use crate::{User, UserDatabase};
use std::path::PathBuf;


/// Returns the path to the user database
pub fn get_db_path() -> Result<PathBuf> {
    let data_dir = crate::get_data_directory()?;
    Ok(data_dir.join("users.sqlite"))
}


/// Initializes the user database and returns the path
pub async fn init_user_database() -> Result<PathBuf> {
    // Determine storage location for the database
    let db_path = get_db_path()?;

    // Create the directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open/create the database (initializes automatically)
    let _db = UserDatabase::open(&db_path).await?;

    println!("User database initialized: {}", db_path.display());

    Ok(db_path)
}


/// Lists all users
pub async fn list_all_users(db_path: &PathBuf) -> Result<Vec<User>> {
    let db = UserDatabase::open(db_path).await?;
    let users = db.list_users().await?;
    Ok(users)
}

// Starts a background Service for every user
/*pub async fn start_all_user_services(db_path: &PathBuf) -> Result<()> {
    let db = UserDatabase::open(db_path).await?;
    let base_path = db.base_data_path().to_path_buf();
    let users = db.list_users().await?;

    for user in users {
        LedgerNode::start_background(base_path.clone(), user.id.clone());
    }

    Ok(())
}
*/