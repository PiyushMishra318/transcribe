use crate::db::{open_db, require_project, sync_profiles};
use crate::voice::{profiles, ProfileStore};
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Input};
use std::io::{self, Write};
use std::num::NonZero;
use std::path::{Path, PathBuf};

pub fn run_interactive(
    project_name: Option<&str>,
    single_profile: Option<&str>,
    no_play: bool,
) -> Result<()> {
    let db = open_db()?;
    let project = require_project(&db, project_name)?;
    let voices = project.voices_path();

    if !voices.join(profiles::PROFILES_FILE).is_file() {
        anyhow::bail!(
            "no profiles.json in {}; run `transcribe profiles build` first",
            voices.display()
        );
    }

    let store = ProfileStore::load(&voices)?;
    if store.profiles.is_empty() {
        anyhow::bail!("no profiles to label");
    }

    let theme = ColorfulTheme::default();
    let total = store.profiles.len();

    let targets: Vec<usize> = if let Some(id) = single_profile {
        let idx = store
            .profiles
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown profile id: {id}"))?;
        vec![idx]
    } else {
        store
            .profiles
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.labeled)
            .map(|(i, _)| i)
            .collect()
    };

    if targets.is_empty() {
        let labeled = store.profiles.iter().filter(|p| p.labeled).count();
        eprintln!("all profiles already labeled ({labeled}/{total})");
        return Ok(());
    }

    eprintln!(
        "labeling {} profile(s) — voices: {}",
        targets.len(),
        voices.display()
    );

    for idx in targets {
        let profile = &store.profiles[idx];
        let id = profile.id.clone();
        let suggested_color = if profile.color.is_empty() {
            profiles::auto_color(idx, total)
        } else {
            profile.color.clone()
        };

        eprintln!("\n--- {} ---", id);
        eprintln!("  clips: {}", profile.clips.join(", "));
        eprintln!("  embeddings: {}", profile.embeddings.len());

        if !no_play {
            play_profile_clips(&voices, profile)?;
        }

        let action: String = Input::with_theme(&theme)
            .with_prompt("Action: [Enter]=label, [s]kip, [q]uit")
            .default(String::new())
            .interact_text()?;

        let action = action.trim().to_lowercase();
        if action == "q" || action == "quit" {
            eprintln!("saving progress and exiting");
            break;
        }
        if action == "s" || action == "skip" {
            eprintln!("skipped {id}");
            continue;
        }

        let name: String = Input::with_theme(&theme)
            .with_prompt("Speaker name (empty to skip)")
            .allow_empty(true)
            .interact_text()?;
        let name = name.trim();
        if name.is_empty() {
            eprintln!("left {id} unlabeled");
            continue;
        }

        let color: String = Input::with_theme(&theme)
            .with_prompt(format!("Color hex (default {suggested_color})"))
            .default(suggested_color.clone())
            .allow_empty(true)
            .interact_text()?;
        let color = color.trim();
        let color = if color.is_empty() {
            suggested_color
        } else {
            color.to_string()
        };

        crate::voice::run_label(&voices, &id, name, Some(&color))?;
        db.upsert_profile(project.id, &id, Some(name), &color, true)?;
        eprintln!("labeled {id} as \"{name}\" ({color})");
    }

    sync_profiles(&db, &project)?;
    let (labeled, total) = db.profile_label_stats(project.id)?;
    eprintln!("\n{labeled}/{total} profiles labeled");
    if labeled < total {
        eprintln!("run `transcribe profiles label` again to finish");
    } else {
        eprintln!("ready — run `transcribe run` to transcribe episodes");
    }
    Ok(())
}

fn play_profile_clips(voices_dir: &Path, profile: &profiles::VoiceProfile) -> Result<()> {
    let clips_dir = ProfileStore::clips_dir(voices_dir);
    let clip_paths: Vec<PathBuf> = profile
        .clips
        .iter()
        .map(|c| clips_dir.join(c))
        .filter(|p| p.is_file())
        .collect();

    if clip_paths.is_empty() {
        eprintln!("  (no clip files to play)");
        return Ok(());
    }

    for path in &clip_paths {
        eprint!(
            "  playing {}... ",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        io::stdout().flush()?;
        if let Err(e) = play_wav(path) {
            eprintln!("failed: {e:#}");
            eprintln!("  tip: use --no-play to skip audio");
        } else {
            eprintln!("done");
        }
    }
    Ok(())
}

fn play_wav(path: &Path) -> Result<()> {
    let reader =
        hound::WavReader::open(path).with_context(|| format!("open {}", path.display()))?;
    let spec = reader.spec();
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    let handle = rodio::DeviceSinkBuilder::open_default_sink()
        .context("open audio output (try --no-play)")?;
    let player = rodio::Player::connect_new(handle.mixer());
    let channels = NonZero::new(spec.channels).context("zero audio channels")?;
    let sample_rate = NonZero::new(spec.sample_rate).context("zero sample rate")?;
    let source = rodio::buffer::SamplesBuffer::new(channels, sample_rate, samples);
    player.append(source);
    player.sleep_until_end();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_project_dirs, open_db};
    use crate::voice::profiles::{ProfileStore, VoiceProfile};
    use tempfile::TempDir;

    #[test]
    fn errors_without_profiles() {
        crate::test_util::with_home(|_home| {
            let work = TempDir::new().unwrap();
            init_project_dirs(work.path()).unwrap();
            let db = open_db().unwrap();
            db.insert_project("lbl-err", work.path(), "voices", None)
                .unwrap();
            db.set_active_project("lbl-err").unwrap();
            assert!(run_interactive(Some("lbl-err"), None, true).is_err());
        });
    }

    #[test]
    fn all_labeled_reports_done() {
        crate::test_util::with_home(|_home| {
            let work = TempDir::new().unwrap();
            init_project_dirs(work.path()).unwrap();
            let db = open_db().unwrap();
            db.insert_project("lbl", work.path(), "voices", None)
                .unwrap();
            db.set_active_project("lbl").unwrap();

            let voices = work.path().join("voices");
            let mut store = ProfileStore::load(&voices).unwrap();
            store.profiles.push(VoiceProfile {
                id: "profile_00".into(),
                name: Some("Done".into()),
                color: "#000000".into(),
                labeled: true,
                embeddings: vec![],
                clips: vec![],
            });
            store.save(&voices).unwrap();

            run_interactive(Some("lbl"), None, true).unwrap();
        });
    }
}
