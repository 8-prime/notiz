use native_db::ToKey;
use native_db::{native_db, Models};
use native_model::native_model;
use native_model::Model;
use serde::{Deserialize, Serialize};

use super::DatabaseUuid;

pub fn define_models(models: &mut Models) -> eyre::Result<()> {
    models.define::<Note>()?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 1, version = 1)]
#[native_db]
pub struct Note {
    #[primary_key]
    pub id: Option<DatabaseUuid>,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub favorite: bool,
}
