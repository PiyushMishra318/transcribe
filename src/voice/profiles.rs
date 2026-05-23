use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sherpa_onnx::SpeakerEmbeddingManager;
use std::path::{Path, PathBuf};

pub const MAX_EMBEDDINGS_PER_PROFILE: usize = 20;
pub const PROFILES_FILE: &str = "profiles.json";
pub const BUILD_STATE_FILE: &str = "build_state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStore {
    pub default_speaker: String,
    pub match_threshold: f32,
    pub profiles: Vec<VoiceProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub color: String,
    pub labeled: bool,
    pub embeddings: Vec<Vec<f32>>,
    #[serde(default)]
    pub clips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildState {
    #[serde(default)]
    pub completed_episodes: Vec<String>,
}

impl ProfileStore {
    pub fn load(voices_dir: &Path) -> Result<Self> {
        let path = voices_dir.join(PROFILES_FILE);
        if path.is_file() {
            let data = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            return serde_json::from_str(&data).context("parse profiles.json");
        }
        Ok(Self {
            default_speaker: "DM".to_string(),
            match_threshold: 0.55,
            profiles: Vec::new(),
        })
    }

    pub fn save(&self, voices_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(voices_dir)?;
        let path = voices_dir.join(PROFILES_FILE);
        let data = serde_json::to_string_pretty(self).context("serialize profiles")?;
        std::fs::write(&path, data).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    pub fn load_build_state(voices_dir: &Path) -> Result<BuildState> {
        let path = voices_dir.join(BUILD_STATE_FILE);
        if !path.is_file() {
            return Ok(BuildState::default());
        }
        let data = std::fs::read_to_string(&path)?;
        serde_json::from_str(&data).context("parse build_state.json")
    }

    pub fn save_build_state(voices_dir: &Path, state: &BuildState) -> Result<()> {
        std::fs::create_dir_all(voices_dir)?;
        let path = voices_dir.join(BUILD_STATE_FILE);
        let data = serde_json::to_string_pretty(state).context("serialize build state")?;
        std::fs::write(&path, data).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    pub fn next_profile_id(&self) -> String {
        let n = self.profiles.len();
        format!("profile_{n:02}")
    }

    pub fn find_profile_mut(&mut self, id: &str) -> Option<&mut VoiceProfile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    pub fn display_name(&self, profile: &VoiceProfile) -> String {
        profile.name.clone().unwrap_or_else(|| profile.id.clone())
    }

    pub fn style_for_speaker(&self, speaker: &str) -> Option<&VoiceProfile> {
        self.profiles
            .iter()
            .find(|p| p.name.as_deref() == Some(speaker) || (!p.labeled && p.id == speaker))
    }

    pub fn add_embedding(&mut self, profile_id: &str, embedding: Vec<f32>) {
        if let Some(p) = self.find_profile_mut(profile_id) {
            if p.embeddings.len() >= MAX_EMBEDDINGS_PER_PROFILE {
                return;
            }
            p.embeddings.push(embedding);
        }
    }

    pub fn create_profile(
        &mut self,
        embedding: Vec<f32>,
        index: usize,
        total_hint: usize,
    ) -> String {
        let id = self.next_profile_id();
        let color = auto_color(index.max(self.profiles.len()), total_hint.max(8));
        self.profiles.push(VoiceProfile {
            id: id.clone(),
            name: None,
            color,
            labeled: false,
            embeddings: vec![embedding],
            clips: Vec::new(),
        });
        id
    }

    pub fn build_manager(&self, dim: i32) -> Option<SpeakerEmbeddingManager> {
        let manager = SpeakerEmbeddingManager::create(dim)?;
        for p in &self.profiles {
            if p.embeddings.is_empty() {
                continue;
            }
            let key = self.display_name(p);
            let _ = manager.add_list(&key, &p.embeddings);
        }
        Some(manager)
    }
}

pub fn auto_color(index: usize, total: usize) -> String {
    let total = total.max(1) as f32;
    let hue = (index as f32 / total) * 360.0;
    let (r, g, b) = hsl_to_rgb(hue, 0.65, 0.55);
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (rp, gp, bp) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((rp + m) * 255.0) as u8,
        ((gp + m) * 255.0) as u8,
        ((bp + m) * 255.0) as u8,
    )
}

/// ASS primary colour uses &HAABBGGRR (BGR bytes).
pub fn hex_to_ass_primary(hex: &str) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return "&H00FFFFFF".to_string();
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    format!("&H00{b:02X}{g:02X}{r:02X}")
}

pub fn ass_style_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl ProfileStore {
    pub fn clips_dir(voices_dir: &Path) -> PathBuf {
        voices_dir.join("clips")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn store_roundtrip_and_helpers() {
        let tmp = TempDir::new().unwrap();
        let voices = tmp.path().join("voices");
        let mut store = ProfileStore::load(&voices).unwrap();
        assert!(store.profiles.is_empty());
        let id = store.create_profile(vec![0.1; 8], 0, 4);
        store.add_embedding(&id, vec![0.2; 8]);
        for _ in 0..MAX_EMBEDDINGS_PER_PROFILE {
            store.add_embedding(&id, vec![0.3; 8]);
        }
        store.save(&voices).unwrap();
        let mut loaded = ProfileStore::load(&voices).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        assert!(loaded.find_profile_mut("missing").is_none());
        let p = &loaded.profiles[0];
        assert_eq!(loaded.display_name(p), id);
        assert!(loaded.style_for_speaker(&id).is_some());
        let _ = auto_color(0, 0);
        let _ = hsl_to_rgb(90.0, 0.5, 0.5);
    }

    #[test]
    fn build_state_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let voices = tmp.path().join("voices");
        let mut state = BuildState::default();
        state.completed_episodes.push("ep1".into());
        ProfileStore::save_build_state(&voices, &state).unwrap();
        let loaded = ProfileStore::load_build_state(&voices).unwrap();
        assert_eq!(loaded.completed_episodes, vec!["ep1"]);
    }
}
