use anyhow::{bail, Context, Result};
use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig,
};
use std::path::Path;

pub struct DiarizedSegment {
    pub start: f32,
    pub end: f32,
    pub _local_speaker: i32,
}

pub struct Diarizer {
    inner: OfflineSpeakerDiarization,
}

impl Diarizer {
    pub fn new(models_dir: &Path, expected_speakers: i32) -> Result<Self> {
        let seg_model = models_dir
            .join("sherpa-onnx-pyannote-segmentation-3-0")
            .join("model.onnx");
        let emb_model = models_dir.join("wespeaker_en_voxceleb_resnet34_LM.onnx");

        if !seg_model.is_file() {
            bail!(
                "segmentation model not found: {}\nDownload:\n  https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2\nExtract under models/",
                seg_model.display()
            );
        }
        if !emb_model.is_file() {
            bail!(
                "embedding model not found: {}\nDownload:\n  https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx\nPlace in models/",
                emb_model.display()
            );
        }

        let config = OfflineSpeakerDiarizationConfig {
            segmentation: OfflineSpeakerSegmentationModelConfig {
                pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                    model: Some(seg_model.to_string_lossy().into_owned()),
                },
                ..Default::default()
            },
            embedding: SpeakerEmbeddingExtractorConfig {
                model: Some(emb_model.to_string_lossy().into_owned()),
                num_threads: 2,
                debug: false,
                provider: Some("cpu".into()),
            },
            clustering: FastClusteringConfig {
                num_clusters: expected_speakers,
                ..Default::default()
            },
            ..Default::default()
        };

        let inner = OfflineSpeakerDiarization::create(&config)
            .context("create offline speaker diarization")?;
        Ok(Self { inner })
    }

    pub fn process(&self, samples: &[f32]) -> Result<Vec<DiarizedSegment>> {
        let result = self.inner.process(samples).context("speaker diarization")?;

        let mut segments = Vec::new();
        for s in result.sort_by_start_time() {
            segments.push(DiarizedSegment {
                start: s.start,
                end: s.end,
                _local_speaker: s.speaker,
            });
        }
        Ok(segments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_models_error() {
        let tmp = TempDir::new().unwrap();
        assert!(Diarizer::new(tmp.path(), 2).is_err());
    }
}
