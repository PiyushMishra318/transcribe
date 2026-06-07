use crate::db::{init_project_dirs, open_db, warn_if_no_videos, Database, Project};
use crate::paths;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run_init(name: &str, path: &Path, models_dir: Option<&Path>) -> Result<()> {
    let db = open_db()?;
    warn_if_no_videos(path)?;
    init_project_dirs(path)?;

    let models = match models_dir {
        Some(p) => p.to_path_buf(),
        None => paths::default_models_dir()?,
    };
    let project = db.insert_project(name, path, "voices", Some(&models))?;
    db.set_active_project(name)?;
    let n = db.scan_episodes_for_project(&project)?;

    eprintln!("registered project \"{name}\"");
    eprintln!("  root: {}", project.root_path.display());
    eprintln!("  voices: {}", project.voices_path().display());
    eprintln!("  models: {}", project.resolved_models_path()?.display());
    eprintln!("  episodes indexed: {n}");
    eprintln!("active project set to \"{name}\"");
    Ok(())
}

pub fn run_list() -> Result<()> {
    let db = open_db()?;
    let active = db.active_project()?;
    let projects = db.list_projects()?;
    if projects.is_empty() {
        eprintln!("no projects registered");
        return Ok(());
    }
    for p in projects {
        let active_mark = active.as_ref().is_some_and(|a| a.id == p.id);
        eprintln!(
            "{}{}  {}",
            if active_mark { "*" } else { " " },
            p.name,
            p.root_path.display()
        );
    }
    Ok(())
}

pub fn run_use(name: &str) -> Result<()> {
    let db = open_db()?;
    let project = db.get_project_by_name(name)?;
    db.set_active_project(name)?;
    eprintln!(
        "active project: \"{}\" ({})",
        name,
        project.root_path.display()
    );
    Ok(())
}

pub fn run_show() -> Result<()> {
    let db = open_db()?;
    let project = db
        .active_project()?
        .ok_or_else(|| anyhow::anyhow!("no active project"))?;

    print_project_details(&db, &project)
}

fn print_project_details(db: &Database, project: &Project) -> Result<()> {
    let (labeled, total) = db.profile_label_stats(project.id)?;
    let voices = project.voices_path();
    let has_profiles = voices.join(crate::voice::profiles::PROFILES_FILE).is_file();

    eprintln!("active project: {}", project.name);
    eprintln!("  root: {}", project.root_path.display());
    eprintln!("  voices: {}", voices.display());
    eprintln!("  models: {}", project.resolved_models_path()?.display());
    eprintln!(
        "  profiles: {labeled}/{total} labeled{}",
        if has_profiles {
            ""
        } else {
            " (no profiles.json yet)"
        }
    );

    let episode_count: usize = db.conn.query_row(
        "SELECT COUNT(*) FROM episodes WHERE project_id = ?1",
        [project.id],
        |row| row.get::<_, i64>(0),
    )? as usize;
    let transcribed: usize = db.conn.query_row(
        "SELECT COUNT(*) FROM episodes WHERE project_id = ?1 AND transcribed_at IS NOT NULL",
        [project.id],
        |row| row.get::<_, i64>(0),
    )? as usize;
    eprintln!("  episodes: {transcribed}/{episode_count} transcribed");
    Ok(())
}

pub fn resolve_models_dir(
    project: Option<&Project>,
    cli_models: Option<PathBuf>,
) -> Result<PathBuf> {
    if let Some(m) = cli_models {
        return Ok(m);
    }
    if let Some(p) = project {
        if let Some(ref stored) = p.models_path {
            return Ok(stored.clone());
        }
    }
    paths::default_models_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use tempfile::TempDir;

    #[test]
    fn project_commands_via_registry() {
        crate::test_util::with_home(|_home| {
            let work = TempDir::new().unwrap();
            std::fs::write(work.path().join("ep.mp4"), b"").unwrap();

            run_init("test-proj", work.path(), None).unwrap();
            run_list().unwrap();
            run_use("test-proj").unwrap();
            run_show().unwrap();
            assert!(run_init("test-proj", work.path(), None).is_err());

            let db = open_db().unwrap();
            let p = db.get_project_by_name("test-proj").unwrap();
            let _ = resolve_models_dir(Some(&p), None).unwrap();
        });
    }
}
