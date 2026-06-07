use crate::paths::discover_videos;
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub struct Database {
    pub conn: Connection,
    pub home: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub root_path: PathBuf,
    pub voices_rel: String,
    pub models_path: Option<PathBuf>,
    pub created_at: String,
}

impl Project {
    pub fn voices_path(&self) -> PathBuf {
        self.root_path.join(&self.voices_rel)
    }

    pub fn resolved_models_path(&self) -> Result<PathBuf> {
        if let Some(ref p) = self.models_path {
            return Ok(p.clone());
        }
        crate::paths::default_models_dir()
    }
}

fn row_to_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    let models: Option<String> = row.get(4)?;
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        root_path: PathBuf::from(row.get::<_, String>(2)?),
        voices_rel: row.get(3)?,
        models_path: models.map(PathBuf::from),
        created_at: row.get(5)?,
    })
}

const PROJECT_COLS: &str = "id, name, root_path, voices_rel, models_path, created_at";

impl Database {
    pub fn insert_project(
        &self,
        name: &str,
        root_path: &Path,
        voices_rel: &str,
        models_path: Option<&Path>,
    ) -> Result<Project> {
        let root = root_path
            .canonicalize()
            .with_context(|| format!("resolve {}", root_path.display()))?;
        let created_at = Utc::now().to_rfc3339();
        let models_str = models_path.map(|p| p.to_string_lossy().into_owned());

        self.conn
            .execute(
                "INSERT INTO projects (name, root_path, voices_rel, models_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    name,
                    root.to_string_lossy().as_ref(),
                    voices_rel,
                    models_str,
                    created_at,
                ],
            )
            .map_err(|e| {
                if let rusqlite::Error::SqliteFailure(err, _) = &e {
                    if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE {
                        return anyhow::anyhow!(
                            "project name already exists: {name} (try {name}-2)"
                        );
                    }
                }
                anyhow::Error::from(e)
            })?;

        let id = self.conn.last_insert_rowid();
        self.get_project_by_id(id)
    }

    pub fn get_project_by_id(&self, id: i64) -> Result<Project> {
        self.conn
            .query_row(
                &format!("SELECT {PROJECT_COLS} FROM projects WHERE id = ?1"),
                [id],
                row_to_project,
            )
            .context("project not found")
    }

    pub fn get_project_by_name(&self, name: &str) -> Result<Project> {
        self.conn
            .query_row(
                &format!("SELECT {PROJECT_COLS} FROM projects WHERE name = ?1"),
                [name],
                row_to_project,
            )
            .with_context(|| format!("project not found: {name}"))
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {PROJECT_COLS} FROM projects ORDER BY name"
        ))?;
        let rows = stmt.query_map([], row_to_project)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("list projects")
    }

    pub fn set_active_project(&self, name: &str) -> Result<()> {
        let project = self.get_project_by_name(name)?;
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES ('active_project_id', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![project.id.to_string()],
        )?;
        let mut config = self.load_config()?;
        config.active_project = Some(name.to_string());
        self.save_config(&config)?;
        Ok(())
    }

    pub fn active_project(&self) -> Result<Option<Project>> {
        let id: Option<i64> = match self.conn.query_row(
            "SELECT value FROM settings WHERE key = 'active_project_id'",
            [],
            |row| {
                let s: String = row.get(0)?;
                Ok(s.parse().ok())
            },
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(e.into()),
        };

        if let Some(id) = id {
            return Ok(Some(self.get_project_by_id(id)?));
        }

        if let Some(name) = self.load_config()?.active_project {
            if let Ok(p) = self.get_project_by_name(&name) {
                return Ok(Some(p));
            }
        }

        Ok(None)
    }

    pub fn scan_episodes_for_project(&self, project: &Project) -> Result<usize> {
        let videos = discover_videos(&project.root_path)?;
        let mut count = 0;
        for video in videos {
            let stem = crate::paths::file_stem_label(&video);
            let file_path = super::sync::episode_file_path(&video);
            self.conn.execute(
                "INSERT INTO episodes (project_id, file_path, stem, profile_built, transcribed_at, last_error)
                 VALUES (?1, ?2, ?3, 0, NULL, NULL)
                 ON CONFLICT(project_id, file_path) DO UPDATE SET stem = excluded.stem",
                params![project.id, file_path, stem],
            )?;
            count += 1;
        }
        Ok(count)
    }

    pub fn profile_label_stats(&self, project_id: i64) -> Result<(usize, usize)> {
        let total: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM profiles WHERE project_id = ?1",
            [project_id],
            |row| row.get::<_, i64>(0),
        )? as usize;
        let labeled: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM profiles WHERE project_id = ?1 AND labeled = 1",
            [project_id],
            |row| row.get::<_, i64>(0),
        )? as usize;
        Ok((labeled, total))
    }

    pub fn unlabeled_profiles(&self, project_id: i64) -> Result<Vec<DbProfile>> {
        let mut stmt = self.conn.prepare(
            "SELECT profile_id, name, color, labeled
             FROM profiles WHERE project_id = ?1 AND labeled = 0
             ORDER BY profile_id",
        )?;
        let rows = stmt.query_map([project_id], |row| {
            Ok(DbProfile {
                profile_id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                labeled: row.get::<_, i32>(3)? != 0,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("unlabeled profiles")
    }

    pub fn get_db_profile(&self, project_id: i64, profile_id: &str) -> Result<Option<DbProfile>> {
        match self.conn.query_row(
            "SELECT profile_id, name, color, labeled
             FROM profiles WHERE project_id = ?1 AND profile_id = ?2",
            params![project_id, profile_id],
            |row| {
                Ok(DbProfile {
                    profile_id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    labeled: row.get::<_, i32>(3)? != 0,
                })
            },
        ) {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_profile(
        &self,
        project_id: i64,
        profile_id: &str,
        name: Option<&str>,
        color: &str,
        labeled: bool,
    ) -> Result<()> {
        let updated_at = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO profiles (project_id, profile_id, name, color, labeled, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(project_id, profile_id) DO UPDATE SET
               name = excluded.name,
               color = excluded.color,
               labeled = excluded.labeled,
               updated_at = excluded.updated_at",
            params![
                project_id,
                profile_id,
                name,
                color,
                labeled as i32,
                updated_at,
            ],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DbProfile {
    pub profile_id: String,
    pub name: Option<String>,
    pub color: Option<String>,
    pub labeled: bool,
}

pub fn init_project_dirs(root: &Path) -> Result<()> {
    let voices = root.join("voices");
    std::fs::create_dir_all(&voices)?;
    std::fs::create_dir_all(voices.join("clips"))?;
    Ok(())
}

pub fn warn_if_no_videos(root: &Path) -> Result<()> {
    let videos = discover_videos(root)?;
    if videos.is_empty() {
        eprintln!(
            "warning: no .mp4 files found in {} (project registered anyway)",
            root.display()
        );
    }
    Ok(())
}

pub fn require_project(db: &Database, name: Option<&str>) -> Result<Project> {
    if let Some(n) = name {
        return db.get_project_by_name(n);
    }
    db.active_project()?.ok_or_else(|| {
        anyhow::anyhow!(
            "no active project; run `transcribe project init` or `transcribe project use`"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use tempfile::TempDir;

    #[test]
    fn project_lifecycle_and_constraints() {
        crate::test_util::with_home(|_home| {
            let work = TempDir::new().unwrap();
            std::fs::write(work.path().join("ep.mp4"), b"").unwrap();
            init_project_dirs(work.path()).unwrap();

            let db = open_db().unwrap();
            let p = db
                .insert_project("alpha", work.path(), "voices", None)
                .unwrap();
            assert!(db
                .insert_project("alpha", work.path(), "voices", None)
                .is_err());
            db.set_active_project("alpha").unwrap();
            let active = db.active_project().unwrap().unwrap();
            assert_eq!(active.id, p.id);
            let list = db.list_projects().unwrap();
            assert_eq!(list.len(), 1);
            db.upsert_profile(p.id, "profile_00", Some("Bob"), "#112233", true)
                .unwrap();
            let (labeled, total) = db.profile_label_stats(p.id).unwrap();
            assert_eq!((labeled, total), (1, 1));
            let _ = db.get_db_profile(p.id, "profile_00").unwrap();
            let _ = db.unlabeled_profiles(p.id).unwrap();
            let _ = db.scan_episodes_for_project(&p).unwrap();
            let _ = p.resolved_models_path().unwrap();
            let _ = p.voices_path();
        });
    }

    #[test]
    fn warn_if_no_videos_still_ok() {
        let tmp = TempDir::new().unwrap();
        warn_if_no_videos(tmp.path()).unwrap();
    }
}
