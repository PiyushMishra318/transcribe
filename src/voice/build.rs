use super::clips;
use super::engine::VoiceEngine;
use super::profiles::{ProfileStore, BUILD_STATE_FILE, PROFILES_FILE};
use super::sample;
use crate::paths::{discover_videos, file_stem_label};
use anyhow::Result;
use std::path::Path;

#[allow(clippy::type_complexity)]
pub fn run_build(
    path: &Path,
    voices_dir: &Path,
    models_dir: &Path,
    sample_minutes: f64,
    expected_speakers: i32,
    force: bool,
    mut on_checkpoint: Option<&mut dyn FnMut(&str) -> Result<()>>,
) -> Result<()> {
    if force {
        let _ = std::fs::remove_file(voices_dir.join(PROFILES_FILE));
        let _ = std::fs::remove_file(voices_dir.join(BUILD_STATE_FILE));
    } else if voices_dir.join(PROFILES_FILE).is_file() {
        eprintln!(
            "profiles.json already exists in {} (use --force to rebuild)",
            voices_dir.display()
        );
    }

    let videos = discover_videos(path)?;
    if videos.is_empty() {
        anyhow::bail!("no .mp4 files found under {}", path.display());
    }

    std::fs::create_dir_all(voices_dir)?;
    let mut engine = VoiceEngine::for_build(models_dir, voices_dir, expected_speakers)?;
    let mut build_state = ProfileStore::load_build_state(voices_dir)?;
    let mut episode_audio: Vec<(String, Vec<f32>, Vec<clips::SegmentRef>)> = Vec::new();

    for video in &videos {
        let stem = file_stem_label(video);
        if !force && build_state.completed_episodes.contains(&stem) {
            eprintln!("skipped (checkpoint): {stem}");
            continue;
        }

        eprintln!("profiling: {stem}...");
        let samples = sample::sample_episode(video, sample_minutes)?;
        let diarizer = engine.diarizer()?;
        let segments = diarizer.process(&samples)?;

        let mut local_segments: Vec<clips::SegmentRef> = Vec::new();
        for seg in segments {
            let dur = (seg.end - seg.start) as f64;
            if dur < 1.0 {
                continue;
            }
            let slice = sample::slice_samples(&samples, seg.start as f64, seg.end as f64);
            let embedding = match engine.embed_samples(&slice) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let profile_id = engine.match_or_create_profile(embedding, expected_speakers as usize);
            local_segments.push((seg.start as f64, seg.end as f64, profile_id));
        }

        episode_audio.push((stem.clone(), samples, local_segments));
        build_state.completed_episodes.push(stem.clone());
        ProfileStore::save_build_state(voices_dir, &build_state)?;
        engine.save_profiles(voices_dir)?;
        if let Some(cb) = on_checkpoint.as_mut() {
            cb(&stem)?;
        }
        eprintln!("checkpoint: {stem}");
    }

    eprintln!("exporting review clips...");
    clips::export_profile_clips(voices_dir, &mut engine.store, &episode_audio)?;
    engine.rebuild_manager();
    engine.save_profiles(voices_dir)?;
    super::review::write_html(voices_dir, &engine.store)?;

    eprintln!(
        "done: {} profile(s) in {}",
        engine.store.profiles.len(),
        voices_dir.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn no_videos_errors() {
        let tmp = TempDir::new().unwrap();
        let voices = tmp.path().join("voices");
        assert!(run_build(tmp.path(), &voices, tmp.path(), 1.0, 2, false, None).is_err());
    }
}
