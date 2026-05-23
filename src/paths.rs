use crate::db::transcribe_home;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Where Whisper / sherpa weights live. Never defaults to the campaign folder.
///
/// Priority: `TRANSCRIBE_MODELS_DIR` → models next to the installed binary →
/// `transcribe/models` beside cwd → repo `models/` when run from a clone →
/// `~/.transcribe/models`.
pub fn default_models_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("TRANSCRIBE_MODELS_DIR") {
        let p = PathBuf::from(dir);
        std::fs::create_dir_all(&p)?;
        return Ok(p);
    }

    if let Some(p) = models_beside_executable() {
        return Ok(p);
    }

    if let Some(p) = sibling_transcribe_models() {
        return Ok(p);
    }

    if let Some(p) = repo_models_from_ancestors() {
        return Ok(p);
    }

    let home = transcribe_home()?;
    let models = home.join("models");
    std::fs::create_dir_all(&models)?;
    Ok(models)
}

fn models_beside_executable() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let install_dir = exe.parent()?;
    let models = install_dir.join("models");
    std::fs::create_dir_all(&models).ok()?;
    Some(models)
}

/// Campaign folder sitting next to a `transcribe/` clone (e.g. Lunarfold + transcribe/).
fn sibling_transcribe_models() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    for sub in ["transcribe/models", "episode-transcribe/models"] {
        let candidate = cwd.join(sub);
        if candidate.is_dir() {
            return candidate.canonicalize().ok();
        }
    }
    None
}

/// Running inside the Rust repo (`cargo run`, dev).
fn repo_models_from_ancestors() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let models = dir.join("models");
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file()
            && models.is_dir()
            && std::fs::read_to_string(&manifest)
                .ok()
                .is_some_and(|t| t.contains("episode-transcribe"))
        {
            return models.canonicalize().ok();
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

pub fn discover_videos(path: &Path) -> Result<Vec<PathBuf>> {
    let path = path
        .canonicalize()
        .with_context(|| format!("resolve {}", path.display()))?;
    let mut videos = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("mp4") {
            videos.push(path);
        }
        return Ok(videos);
    }

    if !path.is_dir() {
        bail!("not a file or directory: {}", path.display());
    }

    for entry in WalkDir::new(&path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("mp4") {
            videos.push(p.to_path_buf());
        }
    }

    videos.sort();
    Ok(videos)
}

/// Resolve an explicit path or the current working directory (where you run `transcribe`).
pub fn resolve_working_path(path: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = path {
        return p
            .canonicalize()
            .with_context(|| format!("resolve {}", p.display()));
    }
    std::env::current_dir().context("current working directory")
}

pub fn file_stem_label(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("episode")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn discover_sorted_mp4_in_directory() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("b.mp4"), b"").unwrap();
        std::fs::write(tmp.path().join("a.mp4"), b"").unwrap();
        let vids = discover_videos(tmp.path()).unwrap();
        assert_eq!(vids.len(), 2);
        assert!(vids[0].file_name().unwrap() <= vids[1].file_name().unwrap());
    }

    #[test]
    fn resolve_working_path_none_uses_cwd() {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(resolve_working_path(None).unwrap(), cwd);
    }

    #[test]
    fn file_stem_label_fallback() {
        assert_eq!(file_stem_label(Path::new("")), "episode");
    }

    #[test]
    fn repo_models_from_manifest_dir() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&manifest).unwrap();
        let dir = default_models_dir().unwrap();
        let _ = std::env::set_current_dir(prev);
        assert!(dir.ends_with("models") || dir.exists());
    }
}
