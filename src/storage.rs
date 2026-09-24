use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Note {
    pub id: i64,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub semantic_enabled: bool,
    pub laya_endpoint: String,
}

pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn open(path: &Path) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        Self::connect(options).await
    }

    async fn connect(options: SqliteConnectOptions) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS notes (\
                id INTEGER PRIMARY KEY, \
                body TEXT NOT NULL DEFAULT '', \
                created_at INTEGER NOT NULL, \
                updated_at INTEGER NOT NULL\
            )",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                semantic_enabled INTEGER NOT NULL DEFAULT 0,
                laya_endpoint TEXT NOT NULL DEFAULT 'http://127.0.0.1:8000/v1/systemone'
            )",
        )
        .execute(&pool)
        .await?;
        sqlx::query("INSERT OR IGNORE INTO settings (id) VALUES (1)")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }

    pub async fn settings(&self) -> Result<Settings, sqlx::Error> {
        let (enabled, endpoint): (i64, String) =
            sqlx::query_as("SELECT semantic_enabled, laya_endpoint FROM settings WHERE id = 1")
                .fetch_one(&self.pool)
                .await?;
        Ok(Settings {
            semantic_enabled: enabled != 0,
            laya_endpoint: endpoint,
        })
    }

    pub async fn save_settings(&self, settings: &Settings) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE settings SET semantic_enabled = ?, laya_endpoint = ? WHERE id = 1")
            .bind(i64::from(settings.semantic_enabled))
            .bind(&settings.laya_endpoint)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Note>, sqlx::Error> {
        sqlx::query_as::<_, Note>("SELECT id, body FROM notes ORDER BY updated_at DESC, id DESC")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn create(&self) -> Result<i64, sqlx::Error> {
        let now = now_millis();
        let result =
            sqlx::query("INSERT INTO notes (body, created_at, updated_at) VALUES ('', ?, ?)")
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn update(&self, id: i64, body: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE notes SET body = ?, updated_at = MAX(updated_at + 1, ?) WHERE id = ?")
            .bind(body)
            .bind(now_millis())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_persist_as_plain_text_and_can_be_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.sqlite3");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let store = Store::open(&path).await.unwrap();
            let id = store.create().await.unwrap();
            let body = "First line\nUnicode: Grüße 🌿\n* still plain text";
            store.update(id, body).await.unwrap();
            let notes = store.list().await.unwrap();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].id, id);
            assert_eq!(notes[0].body, body);
            let settings = Settings {
                semantic_enabled: true,
                laya_endpoint: "http://127.0.0.1:9000/v1/systemone".to_owned(),
            };
            store.save_settings(&settings).await.unwrap();
            drop(store);

            let store = Store::open(&path).await.unwrap();
            assert_eq!(store.list().await.unwrap()[0].body, body);
            assert_eq!(store.settings().await.unwrap(), settings);
            store.delete(id).await.unwrap();
            assert!(store.list().await.unwrap().is_empty());
        });
    }
}
