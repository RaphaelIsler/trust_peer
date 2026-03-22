use crate::user::{Store, ToBackend, ToFrontend, User};
use anyhow::Result;
use std::path::PathBuf;
pub struct Service {}
use db::DbEntity;

impl Service {
    /// Returns the path to the user database
    fn get_db_path() -> Result<PathBuf> {
        let data_dir = crate::get_data_directory()?;
        Ok(data_dir.join("users.sqlite"))
    }

    pub fn new() -> Self {
        Self {}
    }

    pub async fn init(&mut self) -> Result<()> {
        let db_path = Self::get_db_path()?;

        // Create the directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let own = db::DB::open(&db_path).await?;
        own.migrate_table::<User>().await?;

        println!("User database initialized: {}", db_path.display());
        Ok(())
    }

    pub async fn load_users(&self) -> Result<Store> {
        let db_path = Self::get_db_path()?;
        let own = db::DB::open(&db_path).await?;
        let store = User::list(own.connection()).await?;
        Ok(Store::from(store))
    }

    pub async fn handle_frontend(&mut self, message: ToBackend) -> Result<Option<ToFrontend>> {
        let ret = match message {
            ToBackend::Load => Some(ToFrontend::Users(self.load_users().await?)),
            ToBackend::Create(identification) => match identification {
                super::Identification::NameAndBirth {
                    first_name,
                    last_name,
                    middle_name,
                    birth_date,
                } => {
                    let db_path = Self::get_db_path()?;
                    let own = db::DB::open(&db_path).await?;
                    let user = User::new(first_name, last_name);
                    user.write(own.connection()).await?;
                    Some(ToFrontend::Users(self.load_users().await?))
                }
            },
        };
        Ok(ret)
    }
}
