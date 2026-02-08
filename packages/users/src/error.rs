use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Invalid user data: {0}")]
    InvalidData(String),

    #[error("Crypto error: {0}")]
    Crypto(#[from] anyhow::Error),

    #[error("Blockchain error: {0}")]
    Blockchain(String),
}

pub type Result<T> = std::result::Result<T, UserError>;
