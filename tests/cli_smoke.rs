//! CLI smoke tests (no ML models required).

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Phrases expected in transcripts of `test/sample.mp4` (see test/README.md).
const EXPECTED_PHRASES: &[&str] = &["never", "gonna", "give", "let", "down"];

fn bin() -> Command {
    Command::cargo_bin("transcribe").unwrap()
}

fn bin_with_home(home: &std::path::Path) -> Command {
    let mut cmd = bin();
    cmd.env("TRANSCRIBE_HOME", home);
    cmd
}

fn bundled_test_clip() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test/sample.mp4")
}

#[test]
fn version_prints() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("transcribe"));
}

#[test]
fn help_command_general() {
    bin()
        .args(["help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Episode Transcribe"))
        .stdout(predicate::str::contains("project"));
}

#[test]
fn help_command_topics() {
    for topic in ["project", "profiles", "run", "models", "install"] {
        bin()
            .args(["help", topic])
            .assert()
            .success()
            .stdout(predicate::str::is_empty().not());
    }
}

#[test]
fn clap_subcommand_help() {
    bin()
        .args(["project", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("init"));
}

#[test]
fn project_init_list_show_in_temp_dir() {
    let tmp = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    fs::write(tmp.path().join("sample.mp4"), b"not a real video").unwrap();

    bin_with_home(home.path())
        .current_dir(tmp.path())
        .args(["project", "init", "smoke-test", "--path", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("registered project"));

    bin_with_home(home.path())
        .args(["project", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("smoke-test"));

    bin_with_home(home.path())
        .args(["project", "show"])
        .assert()
        .success()
        .stderr(predicate::str::contains("smoke-test"));
}

#[test]
fn project_use_switches_active() {
    let home = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("ep.mp4"), b"x").unwrap();

    bin_with_home(home.path())
        .current_dir(tmp.path())
        .args(["project", "init", "alpha", "--path", "."])
        .assert()
        .success();

    let tmp2 = TempDir::new().unwrap();
    fs::write(tmp2.path().join("ep.mp4"), b"x").unwrap();
    bin_with_home(home.path())
        .current_dir(tmp2.path())
        .args(["project", "init", "beta", "--path", "."])
        .assert()
        .success();

    bin_with_home(home.path())
        .args(["project", "use", "alpha"])
        .assert()
        .success();

    bin_with_home(home.path())
        .args(["project", "show"])
        .assert()
        .success()
        .stderr(predicate::str::contains("alpha"));
}

#[test]
fn uninstall_requires_confirmation() {
    bin()
        .args(["uninstall"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("aborted"));
}

#[test]
fn discover_test_clip_exists() {
    let clip = bundled_test_clip();
    assert!(
        clip.is_file(),
        "missing bundled fixture: {}",
        clip.display()
    );
    assert!(clip.metadata().unwrap().len() > 1_000);
}

/// Full pipeline: profiles build + run (needs models + ffmpeg). Run with:
/// `TRANSCRIBE_E2E=1 cargo test --release -- --ignored`
#[test]
#[ignore]
fn e2e_transcribe_test_clip() {
    let clip = bundled_test_clip();
    assert!(
        clip.is_file(),
        "missing bundled fixture: {}",
        clip.display()
    );

    let home = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    fs::copy(&clip, tmp.path().join("sample.mp4")).unwrap();

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let models = std::env::var("TRANSCRIBE_MODELS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| manifest.join("models"));

    let base_model = models.join("ggml-base.bin");
    if !base_model.is_file() {
        panic!(
            "missing {}; download per models/README.md",
            base_model.display()
        );
    }

    bin_with_home(home.path())
        .current_dir(tmp.path())
        .args([
            "project",
            "init",
            "e2e",
            "--path",
            ".",
            "--models-dir",
            models.to_str().unwrap(),
        ])
        .assert()
        .success();

    bin_with_home(home.path())
        .current_dir(tmp.path())
        .args([
            "profiles",
            "build",
            "--sample-minutes",
            "0.05",
            "--force",
            "--models-dir",
            models.to_str().unwrap(),
        ])
        .assert()
        .success();

    bin_with_home(home.path())
        .current_dir(tmp.path())
        .args([
            "run",
            ".",
            "--model",
            "base",
            "--models-dir",
            models.to_str().unwrap(),
        ])
        .assert()
        .success();

    let txt_path = tmp.path().join("sample.txt");
    let srt_path = tmp.path().join("sample.srt");
    assert!(txt_path.is_file());
    assert!(srt_path.is_file());

    let txt = fs::read_to_string(&txt_path).unwrap();
    assert!(!txt.trim().is_empty(), "transcript .txt is empty");
    let txt_lower = txt.to_ascii_lowercase();
    assert!(
        EXPECTED_PHRASES
            .iter()
            .any(|phrase| txt_lower.contains(phrase)),
        "transcript missing expected phrases {:?}:\n{txt}",
        EXPECTED_PHRASES
    );

    let srt = fs::read_to_string(&srt_path).unwrap();
    assert!(!srt.trim().is_empty(), "subtitle .srt is empty");
}
