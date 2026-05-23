//! Speaker profiles via sherpa-onnx (offline diarization + embeddings).
//!
//! One-time model download into `transcribe/models/` (see `paths::default_models_dir`):
//!
//! - Segmentation tarball:
//!   https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2
//!   → extract to `models/sherpa-onnx-pyannote-segmentation-3-0/model.onnx`
//! - English embedding:
//!   https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx
//!   → `models/wespeaker_en_voxceleb_resnet34_LM.onnx`

pub mod build;
mod clips;
mod diarize;
pub mod engine;
pub mod profiles;
mod review;
pub mod sample;

pub use build::run_build;
pub use engine::VoiceEngine;
pub use profiles::ProfileStore;

pub fn run_list(voices_dir: &std::path::Path) -> anyhow::Result<()> {
    let store = ProfileStore::load(voices_dir)?;
    if store.profiles.is_empty() {
        eprintln!("no profiles in {}", voices_dir.display());
        return Ok(());
    }
    for p in &store.profiles {
        let name = p.name.as_deref().unwrap_or("(unlabeled)");
        let clips = p.clips.join(", ");
        eprintln!(
            "{}  labeled={}  color={}  name={}  embeddings={}  clips=[{}]",
            p.id,
            p.labeled,
            p.color,
            name,
            p.embeddings.len(),
            clips
        );
    }
    Ok(())
}

pub fn run_label(
    voices_dir: &std::path::Path,
    profile_id: &str,
    name: &str,
    color: Option<&str>,
) -> anyhow::Result<()> {
    let mut store = ProfileStore::load(voices_dir)?;
    let idx = store
        .profiles
        .iter()
        .position(|p| p.id == profile_id)
        .ok_or_else(|| anyhow::anyhow!("unknown profile id: {profile_id}"))?;
    let total = store.profiles.len();
    let profile = &mut store.profiles[idx];
    profile.name = Some(name.to_string());
    profile.labeled = true;
    if let Some(c) = color {
        profile.color = c.to_string();
    } else if profile.color.is_empty() {
        profile.color = profiles::auto_color(idx, total);
    }
    let color_out = profile.color.clone();
    store.save(voices_dir)?;
    eprintln!("labeled {profile_id} as \"{name}\" ({color_out})");
    Ok(())
}

pub fn run_review(voices_dir: &std::path::Path) -> anyhow::Result<()> {
    let store = ProfileStore::load(voices_dir)?;
    review::write_html(voices_dir, &store)?;
    eprintln!("wrote {}", voices_dir.join("review.html").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::profiles::{ProfileStore, VoiceProfile};
    use tempfile::TempDir;

    #[test]
    fn run_list_label_review() {
        let tmp = TempDir::new().unwrap();
        run_list(tmp.path()).unwrap();

        let mut store = ProfileStore::load(tmp.path()).unwrap();
        store.profiles.push(VoiceProfile {
            id: "profile_00".into(),
            name: None,
            color: String::new(),
            labeled: false,
            embeddings: vec![],
            clips: vec![],
        });
        store.save(tmp.path()).unwrap();

        run_label(tmp.path(), "profile_00", "Alice", Some("#AABBCC")).unwrap();
        run_review(tmp.path()).unwrap();
        run_list(tmp.path()).unwrap();
    }

    #[test]
    fn run_label_unknown_id_errors() {
        let tmp = TempDir::new().unwrap();
        ProfileStore::load(tmp.path()).unwrap();
        assert!(run_label(tmp.path(), "nope", "x", None).is_err());
    }
}
