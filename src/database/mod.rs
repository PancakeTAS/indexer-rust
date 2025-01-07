use anyhow::{Context, Result};
use definitions::{JetstreamCursor, Record};
use log::info;
use surrealdb::{engine::any::Any, RecordId, Surreal};

pub mod definitions;
pub mod handlers;
pub mod repo_indexer;
mod utils;

/// Connect to the database
pub async fn connect(
    db_endpoint: String,
) -> anyhow::Result<Surreal<Any>> {
    // connect to the database
    info!(target: "indexer", "Connecting to the database at {}", db_endpoint);
    let db = surrealdb::engine::any::connect(db_endpoint).await?;

    // sign in to the server

    definitions::init(&db)
        .await
        .context("Failed to initialize database schema")?;

    Ok(db)
}

/// Fetch the current cursor from the database
pub async fn fetch_cursor(db: &Surreal<Any>, host: &str) -> Result<Option<JetstreamCursor>> {
    let res: Option<JetstreamCursor> = db.select(("cursor", host)).await?;

    Ok(res)
}

/// Write the cursor to the database
pub async fn write_cursor(db: &Surreal<Any>, host: &str, cursor: u64) -> Result<()> {
    let _: Option<Record> = db
        .upsert(("cursor", host))
        .content(JetstreamCursor {
            time_us: (cursor - 10_000_000),
        })
        .await?;

    Ok(())
}

/// Delete a record from the database
async fn delete_record(db: &Surreal<Any>, table: &str, key: &str) -> anyhow::Result<()> {
    let _: Option<Record> = db.delete(RecordId::from_table_key(table, key)).await?;

    Ok(())
}
