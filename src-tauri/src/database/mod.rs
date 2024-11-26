mod v1;

use native_db::{Builder, Database, Key, Models, ToKey};
use once_cell::sync::Lazy;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::path::Path;
use tokio::fs;

use serde::{Deserialize, Serialize};
pub use v1::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DatabaseUuid(uuid::Uuid);

impl DatabaseUuid {
    pub fn new() -> DatabaseUuid {
        DatabaseUuid(uuid::Uuid::new_v4())
    }
}

impl From<uuid::Uuid> for DatabaseUuid {
    fn from(value: uuid::Uuid) -> Self {
        DatabaseUuid(value)
    }
}

impl From<DatabaseUuid> for uuid::Uuid {
    fn from(value: DatabaseUuid) -> Self {
        value.0
    }
}

impl ToKey for DatabaseUuid {
    fn to_key(&self) -> Key {
        Key::new(self.0.as_bytes().to_vec())
    }

    fn key_names() -> Vec<String> {
        vec!["Uuid".to_string()]
    }
}

impl Display for DatabaseUuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    || -> eyre::Result<()> {
        v1::define_models(&mut models)?;
        Ok(())
    }()
    .expect("Failed to define database models");

    models
});

pub async fn init(data_dir: &str) -> eyre::Result<Database<'static>> {
    fs::create_dir_all(data_dir).await?;
    Ok(Builder::new().create(&MODELS, Path::new(data_dir).join("db.redb"))?)
}
