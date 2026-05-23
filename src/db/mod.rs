mod migrate;
mod projects;
mod sync;

pub use projects::{init_project_dirs, require_project, warn_if_no_videos, Database, Project};
pub use sync::{
    mark_episode_error, mark_episode_profile_built, mark_episode_transcribed, sync_episodes,
    sync_profiles,
};

use anyhow::{Context, Result};
use directories::UserDirs;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

pub fn transcribe_home() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("TRANSCRIBE_HOME") {
        return Ok(PathBuf::from(dir));
    }
    let user_dirs = UserDirs::new().context("home directory")?;
    Ok(user_dirs.home_dir().join(".transcribe"))
}

pub fn open_db() -> Result<Database> {
    let home = transcribe_home()?;
    fs::create_dir_all(&home)?;
    let db_path = home.join("projects.db");
    let conn = Connection::open(&db_path).with_context(|| format!("open {}", db_path.display()))?;
    migrate::migrate(&conn)?;
    Ok(Database { conn, home })
}

pub fn config_path(home: &std::path::Path) -> PathBuf {
    home.join("config.json")
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct UserConfig {
    #[serde(default)]
    pub active_project: Option<String>,
}

impl Database {
    pub fn load_config(&self) -> Result<UserConfig> {
        let path = config_path(&self.home);
        if !path.is_file() {
            return Ok(UserConfig::default());
        }
        let data = fs::read_to_string(&path)?;
        serde_json::from_str(&data).context("parse config.json")
    }

    pub fn save_config(&self, config: &UserConfig) -> Result<()> {
        let path = config_path(&self.home);
        let data = serde_json::to_string_pretty(config)?;
        fs::write(&path, data).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn config_roundtrip() {
        crate::test_util::with_home(|home| {
            let db = open_db().unwrap();
            let mut cfg = db.load_config().unwrap();
            cfg.active_project = Some("x".into());
            db.save_config(&cfg).unwrap();
            let loaded = db.load_config().unwrap();
            assert_eq!(loaded.active_project.as_deref(), Some("x"));
            let _ = config_path(home);
            let _ = transcribe_home().unwrap();
        });
    }
}
