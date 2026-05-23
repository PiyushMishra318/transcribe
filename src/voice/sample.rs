use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

const SAMPLE_RATE: u32 = 16_000;

/// Four chunks spread across the episode (0%, 25%, 50%, 75%).
pub fn sample_episode(video: &Path, total_minutes: f64) -> Result<Vec<f32>> {
    let duration = ffprobe_duration(video)?;
    if duration <= 0.0 {
        bail!("invalid duration for {}", video.display());
    }

    let chunk_secs = (total_minutes * 60.0) / 4.0;
    let offsets = [0.0, 0.25, 0.5, 0.75];
    let mut merged = Vec::new();

    for frac in offsets {
        let start = (duration * frac).min((duration - 1.0).max(0.0));
        let chunk = extract_chunk(video, start, chunk_secs)?;
        merged.extend(chunk);
    }

    Ok(merged)
}

pub fn slice_samples(samples: &[f32], start_sec: f64, end_sec: f64) -> Vec<f32> {
    let start = (start_sec * SAMPLE_RATE as f64).round() as usize;
    let end = (end_sec * SAMPLE_RATE as f64).round() as usize;
    let end = end.min(samples.len());
    if start >= end {
        return Vec::new();
    }
    samples[start..end].to_vec()
}

pub fn cs_to_sec(cs: i64) -> f64 {
    cs as f64 * 0.01
}

pub fn ffprobe_duration(path: &Path) -> Result<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path.to_str().context("non-UTF-8 path")?,
        ])
        .output()
        .context("run ffprobe (is it on PATH?)")?;

    if !out.status.success() {
        bail!("ffprobe failed for {}", path.display());
    }

    let s = String::from_utf8_lossy(&out.stdout);
    s.trim()
        .parse::<f64>()
        .with_context(|| format!("parse duration from {:?}", s))
}

fn extract_chunk(video: &Path, start_sec: f64, duration_sec: f64) -> Result<Vec<f32>> {
    let out = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            &format!("{start_sec:.3}"),
            "-t",
            &format!("{duration_sec:.3}"),
            "-i",
            video.to_str().context("non-UTF-8 video")?,
            "-vn",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-f",
            "f32le",
            "pipe:1",
        ])
        .output()
        .context("run ffmpeg")?;

    if !out.status.success() {
        bail!("ffmpeg chunk extract failed for {}", video.display());
    }

    let bytes = out.stdout;
    if bytes.len() % 4 != 0 {
        bail!("unexpected ffmpeg f32le output size");
    }

    let mut samples = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(samples)
}

pub fn write_wav(path: &Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("create {}", path.display()))?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn slice_and_cs_helpers() {
        let samples: Vec<f32> = (0..32_000).map(|i| i as f32).collect();
        assert!(slice_samples(&samples, 0.0, 0.5).len() > 0);
        assert!(slice_samples(&samples, 2.0, 1.0).is_empty());
        assert_eq!(cs_to_sec(100), 1.0);
    }

    #[test]
    fn write_wav_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("t.wav");
        let samples = vec![0.0f32; 100];
        write_wav(&path, &samples).unwrap();
        assert!(path.is_file());
    }

    #[test]
    fn sample_episode_with_fixture() {
        let clip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test/sample.mp4");
        if !clip.is_file() {
            return;
        }
        let audio = sample_episode(&clip, 0.01).unwrap();
        assert!(!audio.is_empty());
        let dur = ffprobe_duration(&clip).unwrap();
        assert!(dur > 0.0);
    }
}
