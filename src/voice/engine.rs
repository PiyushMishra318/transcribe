use super::diarize::Diarizer;
use super::profiles::ProfileStore;
use super::sample;
use crate::gpu::onnx_provider;
use anyhow::{bail, Context, Result};
use sherpa_onnx::{
    SpeakerEmbeddingExtractor, SpeakerEmbeddingExtractorConfig, SpeakerEmbeddingManager,
};
use std::path::Path;

pub struct VoiceEngine {
    extractor: SpeakerEmbeddingExtractor,
    diarizer: Option<Diarizer>,
    pub store: ProfileStore,
    manager: Option<SpeakerEmbeddingManager>,
    dim: i32,
}

impl VoiceEngine {
    pub fn for_transcribe(models_dir: &Path, voices_dir: &Path) -> Result<Self> {
        Self::new(models_dir, voices_dir, false, 8)
    }

    pub fn for_build(models_dir: &Path, voices_dir: &Path, expected_speakers: i32) -> Result<Self> {
        Self::new(models_dir, voices_dir, true, expected_speakers)
    }

    fn new(
        models_dir: &Path,
        voices_dir: &Path,
        with_diarizer: bool,
        expected_speakers: i32,
    ) -> Result<Self> {
        let emb_model = models_dir.join("wespeaker_en_voxceleb_resnet34_LM.onnx");
        if !emb_model.is_file() {
            bail!(
                "embedding model not found: {}\nDownload:\n  https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx",
                emb_model.display()
            );
        }

        let config = SpeakerEmbeddingExtractorConfig {
            model: Some(emb_model.to_string_lossy().into_owned()),
            num_threads: 2,
            debug: false,
            provider: Some(onnx_provider()),
        };

        let extractor =
            SpeakerEmbeddingExtractor::create(&config).context("create embedding extractor")?;
        let dim = extractor.dim();
        let store = ProfileStore::load(voices_dir)?;
        let manager = store.build_manager(dim);
        let diarizer = if with_diarizer {
            Some(Diarizer::new(models_dir, expected_speakers)?)
        } else {
            None
        };

        Ok(Self {
            extractor,
            diarizer,
            store,
            manager,
            dim,
        })
    }

    pub fn diarizer(&self) -> Result<&Diarizer> {
        self.diarizer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("diarizer not initialized"))
    }

    pub fn embed_samples(&self, samples: &[f32]) -> Result<Vec<f32>> {
        if samples.len() < 1600 {
            bail!("audio slice too short for embedding");
        }
        let stream = self
            .extractor
            .create_stream()
            .context("create embedding stream")?;
        stream.accept_waveform(16_000, samples);
        stream.input_finished();
        if !self.extractor.is_ready(&stream) {
            bail!("embedding stream not ready");
        }
        self.extractor.compute(&stream).context("compute embedding")
    }

    pub fn identify_samples(&self, samples: &[f32]) -> String {
        let embedding = match self.embed_samples(samples) {
            Ok(e) => e,
            Err(_) => return self.store.default_speaker.clone(),
        };
        self.identify_embedding(&embedding)
    }

    pub fn identify_range(&self, full_audio: &[f32], start_cs: i64, end_cs: i64) -> String {
        let t0 = sample::cs_to_sec(start_cs);
        let t1 = sample::cs_to_sec(end_cs);
        let slice = sample::slice_samples(full_audio, t0, t1);
        if slice.len() < 1600 {
            return self.store.default_speaker.clone();
        }
        self.identify_samples(&slice)
    }

    fn identify_embedding(&self, embedding: &[f32]) -> String {
        let Some(manager) = &self.manager else {
            return self.store.default_speaker.clone();
        };
        manager
            .search(embedding, self.store.match_threshold)
            .unwrap_or_else(|| self.store.default_speaker.clone())
    }

    pub fn match_or_create_profile(&mut self, embedding: Vec<f32>, total_hint: usize) -> String {
        if let Some(manager) = &self.manager {
            if let Some(name) = manager.search(&embedding, self.store.match_threshold) {
                if let Some(p) = self
                    .store
                    .profiles
                    .iter()
                    .find(|p| p.name.as_deref() == Some(&name) || p.id == name)
                {
                    let id = p.id.clone();
                    self.store.add_embedding(&id, embedding);
                    return id;
                }
            }
        }

        let idx = self.store.profiles.len();
        let id = self.store.create_profile(embedding, idx, total_hint);
        self.rebuild_manager();
        id
    }

    pub fn rebuild_manager(&mut self) {
        self.manager = self.store.build_manager(self.dim);
    }

    pub fn save_profiles(&self, voices_dir: &Path) -> Result<()> {
        self.store.save(voices_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_embedding_model_errors() {
        let tmp = TempDir::new().unwrap();
        let err = VoiceEngine::for_build(tmp.path(), tmp.path(), 2);
        assert!(err.is_err());
    }
}
