//! Process scanner for game detection
//!
//! Scans running processes and matches against arRPC's detectable.json database.

pub mod detectable;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
mod unix;

use crate::types::{DetectableGame, Executable, Pid, ProcessCache, ProcessInfo};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Process scanner for game detection
pub struct ProcessScanner {
    /// Detectable games database
    database: RwLock<Vec<DetectableGame>>,
    /// Process cache
    cache: RwLock<ProcessCache>,
    /// Last database update time
    last_db_update: RwLock<Option<Instant>>,
    /// Database URL
    db_url: String,
}

/// Detected game info
#[derive(Debug, Clone)]
pub struct DetectedGame {
    pub id: String,
    pub name: String,
    pub pid: Pid,
    pub start_time: i64,
}

impl ProcessScanner {
    pub fn new(db_url: String) -> Self {
        Self {
            database: RwLock::new(Vec::new()),
            cache: RwLock::new(ProcessCache::default()),
            last_db_update: RwLock::new(None),
            db_url,
        }
    }

    /// Initialize database (fetch from URL)
    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.update_database().await
    }

    /// Update detectable games database from URL
    async fn update_database(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Fetching detectable.json from {}", self.db_url);

        let response = reqwest::get(&self.db_url).await?;
        let games: Vec<DetectableGame> = response.json().await?;

        info!("Loaded {} detectable games", games.len());
        *self.database.write().await = games;
        *self.last_db_update.write().await = Some(Instant::now());

        Ok(())
    }

    /// Scan for running games
    pub async fn scan(&self) -> Vec<DetectedGame> {
        // Refresh database periodically (every hour)
        if let Some(last_update) = *self.last_db_update.read().await {
            if last_update.elapsed() > Duration::from_secs(3600) {
                let _ = self.update_database().await;
            }
        }

        // Get current processes
        let processes = get_processes().await;
        let database = self.database.read().await;
        let mut cache = self.cache.write().await;

        let mut detected: Vec<DetectedGame> = Vec::new();
        let mut current_ids: Vec<String> = Vec::new();

        for process in &processes {
            let path_lower = process
                .path
                .to_string_lossy()
                .to_lowercase()
                .replace('\\', "/");

            // Generate path segments for matching
            let to_compare = generate_path_variants(&path_lower);

            // Match against database
            for game in database.iter() {
                if let Some(_) = match_executables(&game.executables, &to_compare, &process.args) {
                    current_ids.push(game.id.clone());

                    // Get or create timestamp
                    let start_time =
                        *cache.timestamps.entry(game.id.clone()).or_insert_with(|| {
                            info!("Detected game: {} with PID {}", game.name, process.pid);
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as i64)
                                .unwrap_or(0)
                        });

                    cache.names.insert(game.id.clone(), game.name.clone());
                    cache.pids.insert(game.id.clone(), process.pid);

                    detected.push(DetectedGame {
                        id: game.id.clone(),
                        name: game.name.clone(),
                        pid: process.pid,
                        start_time,
                    });
                    break;
                }
            }
        }

        // Find games that stopped
        let stopped: Vec<String> = cache
            .timestamps
            .keys()
            .filter(|id| !current_ids.contains(id))
            .cloned()
            .collect();

        for id in stopped {
            if let Some(name) = cache.names.get(&id) {
                info!("Lost game: {}", name);
            }
            cache.timestamps.remove(&id);
            cache.names.remove(&id);
            cache.pids.remove(&id);
        }

        detected
    }

    /// Get PID for a stopped game
    pub async fn get_last_pid(&self, app_id: &str) -> Option<Pid> {
        self.cache.read().await.pids.get(app_id).copied()
    }
}

/// Generate path variants for matching (like arRPC)
fn generate_path_variants(path: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let parts: Vec<&str> = path.split('/').collect();

    for i in 1..parts.len() {
        variants.push(parts[parts.len() - i..].join("/"));
    }

    // Add 64-bit variants
    for variant in variants.clone() {
        variants.push(variant.replace("64", ""));
        variants.push(variant.replace(".x64", ""));
        variants.push(variant.replace("x64", ""));
        variants.push(variant.replace("_64", ""));
    }

    variants
}

/// Match executables against path variants
fn match_executables(
    executables: &[Executable],
    to_compare: &[String],
    args: &Option<String>,
) -> Option<()> {
    for exe in executables {
        if exe.is_launcher {
            continue;
        }

        // Check OS filter
        #[cfg(windows)]
        if exe.os.as_deref() != Some("win32") && exe.os.is_some() {
            continue;
        }

        #[cfg(target_os = "macos")]
        if exe.os.as_deref() != Some("darwin") && exe.os.is_some() {
            continue;
        }

        #[cfg(target_os = "linux")]
        if exe.os.as_deref() != Some("linux") && exe.os.is_some() {
            continue;
        }

        // Check if name matches
        let exe_name = exe.name.to_lowercase();
        let is_exact_match = exe_name.starts_with('>');
        let name_to_match = if is_exact_match {
            &exe_name[1..]
        } else {
            &exe_name
        };

        let name_matches = if is_exact_match {
            to_compare
                .first()
                .map(|s| s == name_to_match)
                .unwrap_or(false)
        } else {
            to_compare.iter().any(|s| s == name_to_match)
        };

        if !name_matches {
            continue;
        }

        // Check arguments if required
        if let Some(required_args) = &exe.arguments {
            if let Some(process_args) = args {
                if !process_args.contains(required_args) {
                    continue;
                }
            } else {
                continue;
            }
        }

        return Some(());
    }

    None
}

/// Get running processes (platform-specific)
#[cfg(windows)]
async fn get_processes() -> Vec<ProcessInfo> {
    windows::get_processes()
}

#[cfg(unix)]
async fn get_processes() -> Vec<ProcessInfo> {
    unix::get_processes()
}

#[cfg(not(any(windows, unix)))]
async fn get_processes() -> Vec<ProcessInfo> {
    Vec::new()
}
