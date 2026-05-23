use super::profiles::{ProfileStore, VoiceProfile};
use super::sample;
use anyhow::Result;
use std::path::Path;

const MIN_CLIP_SECS: f64 = 1.5;
const MAX_CLIPS_PER_PROFILE: usize = 5;

pub type SegmentRef = (f64, f64, String);

/// Pick clear-speech segments and export WAV clips for manual review.
pub fn export_profile_clips(
    voices_dir: &Path,
    store: &mut ProfileStore,
    episode_audio: &[(String, Vec<f32>, Vec<SegmentRef>)],
) -> Result<()> {
    let clips_dir = ProfileStore::clips_dir(voices_dir);
    std::fs::create_dir_all(&clips_dir)?;

    let profile_ids: Vec<String> = store.profiles.iter().map(|p| p.id.clone()).collect();
    for id in profile_ids {
        let mut candidates: Vec<(f64, f64, f32, String, Vec<f32>)> = Vec::new();
        for (episode, samples, segments) in episode_audio {
            for (start, end, pid) in segments {
                if pid != &id {
                    continue;
                }
                let dur = end - start;
                if dur < MIN_CLIP_SECS {
                    continue;
                }
                let slice = sample::slice_samples(samples, *start, *end);
                let rms = rms_energy(&slice);
                candidates.push((*start, *end, rms, episode.clone(), slice));
            }
        }

        candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(MAX_CLIPS_PER_PROFILE);

        if let Some(p) = store.find_profile_mut(&id) {
            export_for_profile(voices_dir, p, &candidates)?;
        }
    }

    Ok(())
}

fn export_for_profile(
    voices_dir: &Path,
    profile: &mut VoiceProfile,
    candidates: &[(f64, f64, f32, String, Vec<f32>)],
) -> Result<()> {
    profile.clips.clear();
    let clips_dir = ProfileStore::clips_dir(voices_dir);

    for (i, (_start, _end, _rms, _ep, slice)) in candidates.iter().enumerate() {
        let fname = format!("{}_{:02}.wav", profile.id, i + 1);
        let path = clips_dir.join(&fname);
        sample::write_wav(&path, slice)?;
        profile.clips.push(format!("clips/{fname}"));
    }

    Ok(())
}

fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::profiles::ProfileStore;
    use tempfile::TempDir;

    #[test]
    fn rms_and_export_clips() {
        assert_eq!(rms_energy(&[]), 0.0);
        assert!(rms_energy(&[0.5, -0.5]) > 0.0);

        let tmp = TempDir::new().unwrap();
        let voices = tmp.path().join("voices");
        let mut store = ProfileStore::load(&voices).unwrap();
        let id = store.create_profile(vec![0.0; 4], 0, 2);
        let samples = vec![0.1f32; 16_000];
        let episode_audio = vec![(
            "ep".to_string(),
            samples.clone(),
            vec![(0.0, 2.0, id.clone())],
        )];
        export_profile_clips(&voices, &mut store, &episode_audio).unwrap();
        assert!(!store.profiles[0].clips.is_empty());
    }
}
