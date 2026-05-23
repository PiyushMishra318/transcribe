//! Long-form CLI reference (`transcribe help`).

const GENERAL: &str = r#"Episode Transcribe — batch MP4 transcriber with optional speaker tagging.

USAGE
  transcribe [PATH]                 Transcribe MP4s in PATH (default: current directory)
  transcribe run [PATH]             Same as above
  transcribe <command> [args...]

GLOBAL BEHAVIOUR
  • Source folder = where you run the command (usually cwd).
  • Models live in transcribe/models/ or ~/.transcribe/models/ (not beside MP4s).
  • Project registry: ~/.transcribe/projects.db

TOPICS
  transcribe help project
  transcribe help profiles
  transcribe help run
  transcribe help models
  transcribe help install

OTHER
  transcribe --version
  transcribe uninstall [--purge] [-y]
"#;

const PROJECT: &str = r#"PROJECT — register campaigns and track progress

  transcribe project init <NAME> [--path DIR] [--models-dir DIR]
      Register a folder of episode MP4s. Creates voices/ under the campaign.
      Sets this project as active. Models default to the shared models directory.

  transcribe project list
      List registered projects (* = active).

  transcribe project use <NAME>
      Set the active project (voice profiles path).

  transcribe project show
      Active project: root, voices, models, label progress, episode counts.

EXAMPLE
  cd /path/to/campaign
  transcribe project init my-table
"#;

const PROFILES: &str = r#"PROFILES — speaker diarization and labeling

  transcribe profiles build [PATH] [options]
      Sample MP4s, diarize speakers, write voices/profiles.json.
      --sample-minutes <N>   Audio to sample per episode (default: 18)
      --expected-speakers <N>  Hint for diarization (default: 8)
      --force                Rebuild from scratch
      --project <NAME>       Voice output project (default: active)
      --models-dir <DIR>     Override model location

  transcribe profiles list
      List profile ids, labels, colors, clip paths.

  transcribe profiles label [options]
      Interactive naming; plays clip WAVs in the terminal.
      --profile <ID>         Label one profile only
      --no-play              Skip audio playback
      --project <NAME>

  transcribe profiles review
      Regenerate voices/review.html for browser review.

EXAMPLE
  transcribe profiles build --sample-minutes 2 --force
  transcribe profiles label
"#;

const RUN: &str = r#"RUN — transcribe episodes to .txt / .srt / .ass

  transcribe run [PATH] [options]
  transcribe [PATH] [options]          (shorthand)

      PATH    File or folder of .mp4 files (default: .)

  -f, --force              Re-transcribe even if outputs exist
  -m, --model <NAME>       Whisper model: base, medium, ... (default: medium)
      --models-dir <DIR>   GGML + sherpa models folder
      --no-speakers        Skip speaker tagging
      --project <NAME>     Use registered project's voices/ for tagging

OUTPUT (next to each video)
  Episode.txt   Plain transcript
  Episode.srt   Subtitles
  Episode.ass   Styled subtitles (when speakers enabled)

EXAMPLE
  cd /path/to/campaign
  transcribe run .
  transcribe run . --model base --project my-table
"#;

const MODELS: &str = r#"MODELS — download once, shared across campaigns

  Default search order:
    1. TRANSCRIBE_MODELS_DIR environment variable
    2. models/ next to installed transcribe binary
    3. transcribe/models/ beside your campaign (sibling repo layout)
    4. ~/.transcribe/models/

  Required (Whisper)
    ggml-medium.bin   https://huggingface.co/ggerganov/whisper.cpp/tree/main
    ggml-base.bin     (faster, lower quality; use --model base)

  Optional (VAD)
    ggml-silero-v6.2.0.bin

  Required for speaker tagging
    sherpa-onnx-pyannote-segmentation-3-0/model.onnx
    wespeaker_en_voxceleb_resnet34_LM.onnx

  See models/README.md in the repository for direct links.
"#;

const INSTALL: &str = r#"INSTALL / UNINSTALL

  Windows (GPU, recommended)
    powershell -ExecutionPolicy Bypass -File scripts/install.ps1

  Windows (CPU only)
    powershell -ExecutionPolicy Bypass -File scripts/install.ps1 -CpuOnly

  Linux / macOS
    ./scripts/install.sh [--cpu-only]

  Cargo
    cargo install --path . --locked

  Uninstall
    transcribe uninstall              Remove binary + PATH entry
    transcribe uninstall --purge -y   Also delete ~/.transcribe registry
    scripts/uninstall.ps1             Same (Windows, if CLI missing)
    scripts/uninstall.sh              Same (Unix)

  Tests
    cargo test                        Unit + CLI smoke tests (bundled test/sample.mp4)
    cargo test -- --ignored           Full e2e (requires models)
"#;

pub fn run_help(topic: Option<&str>) {
    let text = match topic.map(|s| s.to_ascii_lowercase()).as_deref() {
        None => GENERAL,
        Some("project" | "projects") => PROJECT,
        Some("profile" | "profiles" | "voice" | "voices") => PROFILES,
        Some("run" | "transcribe") => RUN,
        Some("model" | "models") => MODELS,
        Some("install" | "uninstall") => INSTALL,
        Some(other) => {
            eprintln!("unknown help topic: {other}");
            eprintln!("topics: project, profiles, run, models, install");
            GENERAL
        }
    };
    print!("{text}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_help_topics_print() {
        for topic in [
            None,
            Some("project"),
            Some("profiles"),
            Some("run"),
            Some("models"),
            Some("install"),
            Some("bogus"),
        ] {
            run_help(topic);
        }
    }
}
