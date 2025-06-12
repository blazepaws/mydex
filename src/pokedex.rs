use serde::{Deserialize, Serialize};
use sqlx::{query, MySqlPool};
use tokio::fs;
use tracing::{debug, info};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
struct Pokedex {
    id: String,
    name: String,
    description: String,
    num_entries: i32,
    thumbnail_url: String,
    spritesheet_url: String,
    commit_hash: String,
    entries: serde_json::Value,
}

/// Load the pokedex definitions and add them to our database.
pub async fn update_pokedex_database(db: MySqlPool) -> Result<(), AppError> {

    async fn inner(db: MySqlPool) -> anyhow::Result<()> {

        // Load the pokedex definitions from the file system
        let mut definition_paths = fs::read_dir("data/compiled").await?;
        while let Some(path) = definition_paths.next_entry().await? {
            let is_json_file = match path.file_type().await {
                Ok(t) => t.is_file() && path.file_name().to_string_lossy().ends_with(".json"),
                Err(_) => false,
            };
            if !is_json_file {
                continue;
            }
            let path = path.path();

            let content = fs::read_to_string(&path).await?;
            let definition: Pokedex = serde_json::from_str(content.as_str())?;
            update_pokedex(&db, definition).await?;
        }

        Ok(())
    }

    // Easy error conversion
    inner(db).await.map_err(AppError::startup)
}

/// Updates the pokedex description and entries in the database.
async fn update_pokedex(db: &MySqlPool, pokedex: Pokedex) -> anyhow::Result<()> {

    // Figure out if the database is actually outdated.
    let existing = query!("select commit_hash from pokedex where name = ?", pokedex.name)
        .fetch_optional(db).await?;
    if let Some(existing) = existing {
        if existing.commit_hash >= pokedex.commit_hash {
            debug!("Skipping pokedex update for '{}': Already up-to-date.", pokedex.name);
            return Ok(());
        }
    };
    
    // Database is outdated. Update data or insert.
    info!("Updating pokedex: {}", pokedex.name);
    update_pokedex_table(db, &pokedex).await?;
    
    Ok(())
}

async fn update_pokedex_table(db: &MySqlPool, pokedex: &Pokedex) -> anyhow::Result<()> {
    query!(
        "
        insert into pokedex 
        values (?, ?, ?, ?, ?, ?, ?, ?)
        on duplicate key update
            name = ?,
            description = ?,
            num_entries = ?,
            thumbnail_url = ?,
            spritesheet_url = ?,
            commit_hash = ?,
            entries = ?
        ", 
        // Insert
        pokedex.id,
        pokedex.name,
        pokedex.description, 
        pokedex.num_entries, 
        pokedex.thumbnail_url.as_str(),
        pokedex.spritesheet_url.as_str(),
        pokedex.commit_hash.as_str(),
        pokedex.entries,
        // Update
        pokedex.name,
        pokedex.description, 
        pokedex.num_entries, 
        pokedex.thumbnail_url.as_str(),
        pokedex.spritesheet_url.as_str(),
        pokedex.commit_hash.as_str(),
        pokedex.entries,
    ).execute(db).await?;
    Ok(())
}