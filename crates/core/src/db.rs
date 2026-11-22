use std::{path::Path, str::FromStr, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::Result;

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    /// Opens (creating if needed) the database at `path` in WAL mode and runs migrations.
    ///
    /// # Errors
    /// Fails when the file cannot be opened or a migration fails.
    pub async fn open(path: &Path) -> Result<Db> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5000))
            .foreign_keys(true);
        Db::connect(SqlitePoolOptions::new(), opts).await
    }

    /// A private in-memory database. One connection, kept alive, because each connection would otherwise
    /// get its own empty database.
    ///
    /// # Errors
    /// Fails when a migration fails.
    pub async fn open_in_memory() -> Result<Db> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None);
        Db::connect(pool, opts).await
    }

    async fn connect(pool: SqlitePoolOptions, opts: SqliteConnectOptions) -> Result<Db> {
        let pool = pool.connect_with(opts).await?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(sqlx::Error::from)?;
        crate::palette::map_legacy_colors(&pool).await?;
        Ok(Db { pool })
    }

    /// SQLite's `data_version`: it changes when another connection commits a write.
    ///
    /// shortcut: the value is per connection and the pool may hand out any of them, so a caller that
    /// polls for external writes should hold one connection (`pool.acquire()`) and call the pragma on it.
    ///
    /// # Errors
    /// Fails when the pragma query fails.
    pub async fn data_version(&self) -> Result<i64> {
        Ok(sqlx::query_scalar("PRAGMA data_version")
            .fetch_one(&self.pool)
            .await?)
    }
}
