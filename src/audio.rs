use anyhow::{bail, Context, Result};
use crate::gpu::ffmpeg_hwaccel_args;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extract 16 kHz mono PCM WAV beside the video, then load as normalized f32 samples.
pub fn extract_and_load(video: &Path) -> Result<(Vec<f32>, PathBuf)> {
    let wav_path = video.with_extension("wav");
    run_ffmpeg(video, &wav_path)?;
    let samples = load_wav(&wav_path)?;
    Ok((samples, wav_path))
}

fn run_ffmpeg(input: &Path, output: &Path) -> Result<()> {
    let mut command = Command::new("ffmpeg");
    command.arg("-y");
    for arg in ffmpeg_hwaccel_args() {
        command.arg(arg);
    }
    let status = command
        .args([
            "-i",
            input.to_str().context("non-UTF-8 video path")?,
            "-vn",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            output.to_str().context("non-UTF-8 wav path")?,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("failed to run ffmpeg (is it on PATH?)")?;

    if !status.success() {
        bail!("ffmpeg exited with {}", status);
    }
    Ok(())
}

fn load_wav(path: &Path) -> Result<Vec<f32>> {
    let reader =
        hound::WavReader::open(path).with_context(|| format!("open {}", path.display()))?;
    let spec = reader.spec();

    if spec.sample_rate != 16_000 {
        bail!(
            "expected 16 kHz WAV, got {} Hz ({})",
            spec.sample_rate,
            path.display()
        );
    }
    if spec.channels != 1 {
        bail!(
            "expected mono WAV, got {} channels ({})",
            spec.channels,
            path.display()
        );
    }

    let samples: Vec<i16> = reader
        .into_samples::<i16>()
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("read WAV samples")?;

    let mut audio = vec![0.0f32; samples.len()];
    whisper_rs::convert_integer_to_float_audio(&samples, &mut audio).context("normalize audio")?;
    Ok(audio)
}

pub fn remove_temp_wav(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};
    use tempfile::TempDir;

    fn write_test_wav(path: &Path) {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut w = WavWriter::create(path, spec).unwrap();
        for _ in 0..16_000 {
            w.write_sample(0i16).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn load_wav_validates_format() {
        let tmp = TempDir::new().unwrap();
        let ok = tmp.path().join("ok.wav");
        write_test_wav(&ok);
        let samples = load_wav(&ok).unwrap();
        assert_eq!(samples.len(), 16_000);

        let bad_rate = tmp.path().join("bad.wav");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut w = WavWriter::create(&bad_rate, spec).unwrap();
        w.write_sample(0i16).unwrap();
        w.finalize().unwrap();
        assert!(load_wav(&bad_rate).is_err());

        let stereo = tmp.path().join("stereo.wav");
        let stereo_spec = WavSpec {
            channels: 2,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut w = WavWriter::create(&stereo, stereo_spec).unwrap();
        w.write_sample(0i16).unwrap();
        w.write_sample(0i16).unwrap();
        w.finalize().unwrap();
        assert!(load_wav(&stereo).is_err());
    }

    #[test]
    fn extract_and_load_sample_clip() {
        let clip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test/sample.mp4");
        if !clip.is_file() {
            return;
        }
        let (samples, wav) = extract_and_load(&clip).unwrap();
        assert!(!samples.is_empty());
        remove_temp_wav(&wav);
        assert!(!wav.exists());
    }
}
