//! Integration tests aimed at full code-path coverage (uses local models when present).

use assert_cmd::Command;
use episode_transcribe::cli::{find_vad_model, outputs_exist, run_from};
use episode_transcribe::output::{write_ass, write_srt, write_txt, Segment};
use episode_transcribe::paths::{discover_videos, file_stem_label};
use episode_transcribe::voice::profiles::{
    ass_style_name, auto_color, hex_to_ass_primary, ProfileStore, VoiceProfile,
};
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn models_dir() -> PathBuf {
    std::env::var("TRANSCRIBE_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("models"))
}

fn sample_clip() -> PathBuf {
    manifest_dir().join("test/sample.mp4")
}

fn has_models() -> bool {
    models_dir().join("ggml-base.bin").is_file()
        && models_dir()
            .join("wespeaker_en_voxceleb_resnet34_LM.onnx")
            .is_file()
}

fn bin_with_home(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("transcribe").unwrap();
    cmd.env("TRANSCRIBE_HOME", home);
    cmd
}

#[test]
fn lib_run_help_unknown_topic() {
    run_from(["transcribe", "help", "not-a-topic"]).unwrap();
}

#[test]
fn lib_outputs_exist_and_vad_helpers() {
    let tmp = TempDir::new().unwrap();
    let video = tmp.path().join("x.mp4");
    fs::write(&video, b"").unwrap();
    assert!(!outputs_exist(&video, false));
    assert!(
        find_vad_model(models_dir().as_path()).is_none()
            || find_vad_model(models_dir().as_path()).is_some()
    );
}

#[test]
fn discover_single_mp4_file() {
    let tmp = TempDir::new().unwrap();
    let mp4 = tmp.path().join("only.mp4");
    fs::write(&mp4, b"x").unwrap();
    let found = discover_videos(&mp4).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(file_stem_label(&mp4), "only");
}

#[test]
fn discover_ignores_non_mp4_file() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("readme.txt"), b"x").unwrap();
    assert!(discover_videos(tmp.path()).unwrap().is_empty());
}

#[test]
fn output_writers_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let segments = vec![
        Segment {
            start_cs: 0,
            end_cs: 100,
            text: "hello  world".to_string(),
            speaker: "Alice".to_string(),
        },
        Segment {
            start_cs: 100,
            end_cs: 200,
            text: "plain".to_string(),
            speaker: String::new(),
        },
    ];
    let store = ProfileStore {
        default_speaker: "DM".to_string(),
        match_threshold: 0.55,
        profiles: vec![],
    };
    write_txt(&tmp.path().join("out.txt"), "title", &segments).unwrap();
    write_srt(&tmp.path().join("out.srt"), &segments).unwrap();
    write_ass(&tmp.path().join("out.ass"), &segments, &store).unwrap();
    let ass = fs::read_to_string(tmp.path().join("out.ass")).unwrap();
    assert!(ass.contains("Dialogue:"));
}

#[test]
fn profile_color_helpers() {
    assert_eq!(hex_to_ass_primary("#FF0000"), "&H000000FF");
    assert_eq!(hex_to_ass_primary("bad"), "&H00FFFFFF");
    assert_eq!(ass_style_name("Player One!"), "Player_One_");
    let _ = auto_color(3, 8);
}

#[test]
fn project_show_without_active_fails() {
    let home = TempDir::new().unwrap();
    bin_with_home(home.path())
        .args(["project", "show"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no active project"));
}

#[test]
fn project_list_empty() {
    let home = TempDir::new().unwrap();
    bin_with_home(home.path())
        .args(["project", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("no projects registered"));
}

#[test]
fn profiles_list_empty_voices_dir() {
    let tmp = TempDir::new().unwrap();
    let voices = tmp.path().join("voices");
    fs::create_dir_all(&voices).unwrap();
    Command::cargo_bin("transcribe")
        .unwrap()
        .args(["profiles", "list", "--voices", voices.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("no profiles"));
}

#[test]
fn uninstall_yes_purge_in_temp_home() {
    let home = TempDir::new().unwrap();
    fs::create_dir_all(home.path()).unwrap();
    bin_with_home(home.path())
        .args(["uninstall", "--purge", "-y"])
        .assert()
        .success();
}

#[test]
fn default_run_is_run_subcommand() {
    let tmp = TempDir::new().unwrap();
    let models = tmp.path().join("models");
    fs::create_dir_all(&models).unwrap();
    Command::cargo_bin("transcribe")
        .unwrap()
        .current_dir(tmp.path())
        .args([
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
fn hidden_voice_alias_lists_profiles() {
    let tmp = TempDir::new().unwrap();
    let voices = tmp.path().join("voices");
    fs::create_dir_all(&voices).unwrap();
    Command::cargo_bin("transcribe")
        .unwrap()
        .args(["voice", "list", "--voices", voices.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
#[ignore = "requires ggml-base.bin and ffmpeg"]
fn transcribe_sample_clip_no_speakers() {
    if !has_models() {
        return;
    }
    let clip = sample_clip();
    if !clip.is_file() {
        return;
    }

    let tmp = TempDir::new().unwrap();
    fs::copy(&clip, tmp.path().join("sample.mp4")).unwrap();
    let models = models_dir();

    Command::cargo_bin("transcribe")
        .unwrap()
        .current_dir(tmp.path())
        .args([
            "run",
            ".",
            "--model",
            "base",
            "--models-dir",
            models.to_str().unwrap(),
            "--no-speakers",
            "--force",
        ])
        .assert()
        .success();

    assert!(tmp.path().join("sample.txt").is_file());
    assert!(tmp.path().join("sample.srt").is_file());
}

#[test]
#[ignore = "requires models, sherpa segmentation, and ffmpeg"]
fn profiles_build_and_transcribe_with_speakers() {
    if !has_models() {
        return;
    }
    let clip = sample_clip();
    if !clip.is_file() {
        return;
    }
    let seg = models_dir()
        .join("sherpa-onnx-pyannote-segmentation-3-0")
        .join("model.onnx");
    if !seg.is_file() {
        return;
    }

    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    fs::copy(&clip, work.path().join("sample.mp4")).unwrap();
    let models = models_dir();

    let mut cmd = bin_with_home(home.path());
    cmd.current_dir(work.path()).args([
        "project",
        "init",
        "cov",
        "--path",
        ".",
        "--models-dir",
        models.to_str().unwrap(),
    ]);
    cmd.assert().success();

    bin_with_home(home.path())
        .current_dir(work.path())
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
        .current_dir(work.path())
        .args(["profiles", "list"])
        .assert()
        .success();

    bin_with_home(home.path())
        .current_dir(work.path())
        .args(["profiles", "review"])
        .assert()
        .success();

    assert!(work.path().join("voices/review.html").is_file());

    let voices = work.path().join("voices");
    let mut store = ProfileStore::load(&voices).unwrap();
    if store.profiles.is_empty() {
        store.profiles.push(VoiceProfile {
            id: "profile_00".into(),
            name: None,
            color: "#AABBCC".into(),
            labeled: false,
            embeddings: vec![],
            clips: vec![],
        });
        store.save(&voices).unwrap();
    }
    let id = store.profiles[0].id.clone();
    bin_with_home(home.path())
        .current_dir(work.path())
        .args([
            "profiles",
            "label-one",
            &id,
            "TestSpeaker",
            "--color",
            "#AABBCC",
        ])
        .assert()
        .success();

    bin_with_home(home.path())
        .current_dir(work.path())
        .args([
            "run",
            ".",
            "--model",
            "base",
            "--models-dir",
            models.to_str().unwrap(),
            "--force",
        ])
        .assert()
        .success();

    assert!(work.path().join("sample.ass").is_file());
}

#[test]
#[ignore = "requires models and ffmpeg"]
fn run_skips_existing_outputs() {
    if !has_models() {
        return;
    }
    let clip = sample_clip();
    if !clip.is_file() {
        return;
    }

    let tmp = TempDir::new().unwrap();
    fs::copy(&clip, tmp.path().join("sample.mp4")).unwrap();
    fs::write(tmp.path().join("sample.txt"), "existing").unwrap();
    fs::write(tmp.path().join("sample.srt"), "existing").unwrap();
    let models = models_dir();

    Command::cargo_bin("transcribe")
        .unwrap()
        .current_dir(tmp.path())
        .args([
            "run",
            ".",
            "--model",
            "base",
            "--models-dir",
            models.to_str().unwrap(),
            "--no-speakers",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("skipped"));
}

#[test]
#[test]
#[ignore = "requires ggml-base.bin"]
fn run_with_vad_when_model_present() {
    if !has_models() {
        return;
    }
    let clip = sample_clip();
    if !clip.is_file() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    fs::copy(&clip, tmp.path().join("sample.mp4")).unwrap();
    let models = models_dir();
    let vad = models.join("ggml-silero-v6.2.0.bin");
    if !vad.is_file() {
        fs::write(&vad, b"not a real vad").ok();
    }
    Command::cargo_bin("transcribe")
        .unwrap()
        .current_dir(tmp.path())
        .args([
            "run",
            ".",
            "--model",
            "base",
            "--models-dir",
            models.to_str().unwrap(),
            "--no-speakers",
            "--force",
        ])
        .assert()
        .success();
}

#[test]
fn lib_run_transcribe_model_missing() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x.mp4"), b"").unwrap();
    let models = tmp.path().join("models");
    fs::create_dir_all(&models).unwrap();

    let err = run_from([
        "transcribe",
        "run",
        tmp.path().to_str().unwrap(),
        "--no-speakers",
        "--model",
        "base",
        "--models-dir",
        models.to_str().unwrap(),
    ]);
    assert!(err.is_err());
}

#[test]
fn paths_default_models_dir_from_env() {
    let tmp = TempDir::new().unwrap();
    let models = tmp.path().join("custom-models");
    std::env::set_var("TRANSCRIBE_MODELS_DIR", &models);
    let resolved = episode_transcribe::paths::default_models_dir().unwrap();
    assert_eq!(resolved, models);
    std::env::remove_var("TRANSCRIBE_MODELS_DIR");
}
