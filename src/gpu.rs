//! GPU defaults for Whisper (CUDA) and sherpa-onnx (CUDA provider when built with cuda).

use whisper_rs::WhisperContextParameters;

/// ONNX Runtime execution provider for sherpa-onnx models.
pub fn onnx_provider() -> String {
    std::env::var("TRANSCRIBE_ONNX_PROVIDER").unwrap_or_else(|_| {
        #[cfg(feature = "cuda")]
        {
            "cuda".into()
        }
        #[cfg(not(feature = "cuda"))]
        {
            "cpu".into()
        }
    })
}

pub fn whisper_context_params() -> WhisperContextParameters<'static> {
    let mut params = WhisperContextParameters::default();
    #[cfg(feature = "cuda")]
    {
        if !gpu_disabled() {
            params.use_gpu(true);
        }
    }
    params
}

pub fn gpu_disabled() -> bool {
    matches!(
        std::env::var("TRANSCRIBE_CPU").ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

/// CUDA decode for ffmpeg when GPU is enabled (mirrors vod-guru ffmpeg_gpu).
pub fn ffmpeg_hwaccel_args() -> &'static [&'static str] {
    if gpu_disabled() {
        return &[];
    }
    &["-hwaccel", "cuda"]
}

pub fn log_whisper_backend() {
    #[cfg(feature = "cuda")]
    {
        if gpu_disabled() {
            eprintln!("whisper: CPU (TRANSCRIBE_CPU set)");
        } else {
            eprintln!("whisper: GPU (CUDA)");
        }
    }
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("whisper: CPU (cuda feature not enabled)");
    }
    let onnx = onnx_provider();
    eprintln!("sherpa-onnx provider: {onnx}");
}
