//! Detectable games database loader
//!
//! Dynamically loads arRPC's detectable.json from GitHub.

use crate::types::DetectableGame;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

const DEFAULT_DB_URL: &str =
    "https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json";
const CACHE_FILE: &str = "detectable_cache.json";

/// Load detectable games database
pub async fn load_database(
    url: &str,
) -> Result<Vec<DetectableGame>, Box<dyn std::error::Error + Send + Sync>> {
    // Try to fetch from URL
    match fetch_from_url(url).await {
        Ok(games) => {
            // Cache for offline use
            let _ = cache_database(&games).await;
            Ok(games)
        }
        Err(e) => {
            warn!("Failed to fetch database from URL: {}", e);
            // Fall back to cache
            load_from_cache().await
        }
    }
}

async fn fetch_from_url(
    url: &str,
) -> Result<Vec<DetectableGame>, Box<dyn std::error::Error + Send + Sync>> {
    info!("Fetching detectable database from {}", url);
    let response = reqwest::get(url).await?;
    let games: Vec<DetectableGame> = response.json().await?;
    info!("Loaded {} detectable games from URL", games.len());
    Ok(games)
}

async fn cache_database(
    games: &[DetectableGame],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cache_path = get_cache_path();
    let json = serde_json::to_string(games)?;
    fs::write(&cache_path, json).await?;
    debug!("Cached database to {:?}", cache_path);
    Ok(())
}

async fn load_from_cache() -> Result<Vec<DetectableGame>, Box<dyn std::error::Error + Send + Sync>>
{
    let cache_path = get_cache_path();
    info!("Loading database from cache: {:?}", cache_path);
    let json = fs::read_to_string(&cache_path).await?;
    let games: Vec<DetectableGame> = serde_json::from_str(&json)?;
    info!("Loaded {} detectable games from cache", games.len());
    Ok(games)
}

fn get_cache_path() -> PathBuf {
    // Use temp directory for cache
    std::env::temp_dir().join(CACHE_FILE)
}
