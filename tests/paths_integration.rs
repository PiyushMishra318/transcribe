//! Path resolution tests via the CLI.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn run_without_mp4_fails_cleanly() {
    let tmp = TempDir::new().unwrap();
    let models = tmp.path().join("models");
    fs::create_dir_all(&models).unwrap();
    // Satisfy model path check so discovery runs and reports missing MP4s.
    fs::write(models.join("ggml-base.bin"), b"not a real model").unwrap();

    Command::cargo_bin("transcribe")
        .unwrap()
        .current_dir(tmp.path())
        .args([
            "run",
            ".",
            "--no-speakers",
            "--model",
            "base",
            "--models-dir",
            models.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no .mp4"));
}

#[test]
fn run_skips_when_outputs_exist() {
    let tmp = TempDir::new().unwrap();
    let mp4 = tmp.path().join("short.mp4");
    fs::write(&mp4, b"x").unwrap();
    fs::write(tmp.path().join("short.txt"), "existing").unwrap();
    fs::write(tmp.path().join("short.srt"), "existing").unwrap();
    fs::write(tmp.path().join("short.words.json"), "{}").unwrap();

    let models = tmp.path().join("models");
    fs::create_dir_all(&models).unwrap();
    let real_base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models/ggml-base.bin");
    if real_base.is_file() {
        fs::copy(&real_base, models.join("ggml-base.bin")).unwrap();
    } else {
        fs::write(models.join("ggml-base.bin"), b"not a real model").unwrap();
    }

    let mut cmd = Command::cargo_bin("transcribe").unwrap();
    cmd.current_dir(tmp.path()).args([
        "run",
        ".",
        "--no-speakers",
        "--model",
        "base",
        "--models-dir",
        models.to_str().unwrap(),
    ]);

    if models
        .join("ggml-base.bin")
        .metadata()
        .map(|m| m.len() > 1_000_000)
        .unwrap_or(false)
    {
        cmd.assert()
            .success()
            .stderr(predicate::str::contains("skipped"));
    } else {
        cmd.assert().failure();
    }
}
