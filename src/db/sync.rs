use super::{Database, Project};
use crate::paths::{discover_videos, file_stem_label};
use crate::voice::profiles::ProfileStore;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
use std::path::Path;

pub fn sync_profiles(db: &Database, project: &Project) -> Result<()> {
    let voices = project.voices_path();
    if !voices.join(crate::voice::profiles::PROFILES_FILE).is_file() {
        return Ok(());
    }
    let store = ProfileStore::load(&voices)?;
    for p in &store.profiles {
        db.upsert_profile(project.id, &p.id, p.name.as_deref(), &p.color, p.labeled)?;
    }
    Ok(())
}

pub fn episode_file_path(video: &Path) -> String {
    video
        .canonicalize()
        .unwrap_or_else(|_| video.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

pub fn sync_episodes(db: &Database, project: &Project, scan_root: &Path) -> Result<()> {
    let videos = discover_videos(scan_root)?;
    for video in videos {
        let stem = file_stem_label(&video);
        let file_path = episode_file_path(&video);
        db.conn.execute(
            "INSERT INTO episodes (project_id, file_path, stem, profile_built, transcribed_at, last_error)
             VALUES (?1, ?2, ?3, 0, NULL, NULL)
             ON CONFLICT(project_id, file_path) DO UPDATE SET stem = excluded.stem",
            params![project.id, file_path, stem],
        )?;
    }
    Ok(())
}

pub fn mark_episode_profile_built(db: &Database, project_id: i64, stem: &str) -> Result<()> {
    db.conn.execute(
        "UPDATE episodes SET profile_built = 1 WHERE project_id = ?1 AND stem = ?2",
        params![project_id, stem],
    )?;
    Ok(())
}

pub fn mark_episode_transcribed(db: &Database, project_id: i64, video: &Path) -> Result<()> {
    let file_path = episode_file_path(video);
    let at = Utc::now().to_rfc3339();
    db.conn.execute(
        "UPDATE episodes SET transcribed_at = ?3, last_error = NULL
         WHERE project_id = ?1 AND file_path = ?2",
        params![project_id, file_path, at],
    )?;
    Ok(())
}

pub fn mark_episode_error(db: &Database, project_id: i64, video: &Path, error: &str) -> Result<()> {
    let file_path = episode_file_path(video);
    db.conn.execute(
        "UPDATE episodes SET last_error = ?3 WHERE project_id = ?1 AND file_path = ?2",
        params![project_id, file_path, error],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_project_dirs, open_db, Database};
    use tempfile::TempDir;

    #[test]
    fn sync_and_mark_episodes() {
        crate::test_util::with_home(|_home| {
            let db = open_db().unwrap();
            let work = TempDir::new().unwrap();
            std::fs::write(work.path().join("ep.mp4"), b"").unwrap();
            init_project_dirs(work.path()).unwrap();
            let project = db.insert_project("p", work.path(), "voices", None).unwrap();
            sync_episodes(&db, &project, work.path()).unwrap();
            let video = work.path().join("ep.mp4");
            mark_episode_profile_built(&db, project.id, "ep").unwrap();
            mark_episode_transcribed(&db, project.id, &video).unwrap();
            mark_episode_error(&db, project.id, &video, "oops").unwrap();
            let _ = episode_file_path(&video);
        });
    }

    #[test]
    fn sync_profiles_no_file_is_ok() {
        crate::test_util::with_home(|_home| {
            let db = open_db().unwrap();
            let work = TempDir::new().unwrap();
            init_project_dirs(work.path()).unwrap();
            let project = db
                .insert_project("p2", work.path(), "voices", None)
                .unwrap();
            sync_profiles(&db, &project).unwrap();
        });
    }
}
