//! Additional integration tests for uncovered branches (no sherpa inference).

use episode_transcribe::cli::run_from;
use episode_transcribe::paths::{default_models_dir, discover_videos, resolve_working_path};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn paths_sibling_layout() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("TRANSCRIBE_MODELS_DIR");
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("transcribe/models")).unwrap();
    fs::create_dir_all(tmp.path().join("campaign")).unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path().join("campaign")).unwrap();
    let dir = default_models_dir().unwrap();
    let _ = std::env::set_current_dir(prev);
    assert!(dir.ends_with("models"));
}

#[test]
fn paths_transcribe_models_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    std::env::set_var("TRANSCRIBE_MODELS_DIR", tmp.path());
    let _ = default_models_dir().unwrap();
    std::env::remove_var("TRANSCRIBE_MODELS_DIR");
}

#[test]
fn discover_ignores_non_mp4_file() {
    let tmp = TempDir::new().unwrap();
    let f = tmp.path().join("readme.txt");
    fs::write(&f, b"x").unwrap();
    assert!(discover_videos(&f).unwrap().is_empty());
}

#[test]
fn resolve_missing_path_errors() {
    assert!(resolve_working_path(Some(Path::new("Z:\\no-such-path-xyz-123"))).is_err());
}

#[test]
fn run_transcribe_whisper_failure_marks_error() {
    let _guard = ENV_LOCK.lock().unwrap();
    let models = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models/ggml-base.bin");
    if !models.is_file() || models.metadata().map(|m| m.len()).unwrap_or(0) < 1_000_000 {
        return;
    }
    let home = TempDir::new().unwrap();
    std::env::set_var("TRANSCRIBE_HOME", home.path());
    let work = TempDir::new().unwrap();
    fs::write(work.path().join("bad.mp4"), b"not video").unwrap();
    let model_dir = TempDir::new().unwrap();
    fs::copy(&models, model_dir.path().join("ggml-base.bin")).unwrap();

    let err = run_from([
        "transcribe",
        "run",
        work.path().to_str().unwrap(),
        "--no-speakers",
        "--model",
        "base",
        "--models-dir",
        model_dir.path().to_str().unwrap(),
        "--force",
    ]);
    assert!(err.is_err());
    std::env::remove_var("TRANSCRIBE_HOME");
}
