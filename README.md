# Episode Transcribe

Batch-transcribe tabletop / actual-play episode MP4s into subtitles (`.srt`, `.txt`, optional speaker-tagged `.ass`) using [whisper.cpp](https://github.com/ggerganov/whisper.cpp) and [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) speaker diarization.

The CLI is **directory-oriented**: install once, then `cd` into whichever folder contains your source MP4s. That folder is the source — outputs are written next to each video.

```bash
transcribe help              # full command reference
transcribe help profiles     # one topic
```

## Minimum system requirements

| | Minimum | Recommended |
|---|---------|-------------|
| **OS** | Windows 10/11, Ubuntu 22.04+, or macOS 12+ | Same |
| **CPU** | 4 cores, x64 | 8+ cores |
| **RAM** | 8 GB | 16 GB+ |
| **Disk** | 3 GB free (models + build) | SSD with 5 GB+ |
| **GPU** (optional) | NVIDIA GPU, 4 GB VRAM, CUDA 12+ toolkit | RTX 3060+ , 8 GB VRAM |
| **Software** | [Rust](https://rustup.rs/) 1.75+, [ffmpeg](https://ffmpeg.org/) on `PATH` | Latest stable Rust, ffmpeg 6+ |

**Notes**

- **CPU-only** builds: `install.ps1 -CpuOnly` or `install.sh --cpu-only`.
- **CUDA build** (default on Windows): MSVC, [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads), [LLVM](https://releases.llvm.org/) (`LIBCLANG_PATH`). Use `scripts/build-cuda.ps1` if `install.ps1` fails.
- Whisper `medium` ~1.5 GB; `base` ~150 MB — see [models/README.md](models/README.md).

## Install

### Windows (GPU)

```powershell
git clone https://github.com/your-org/transcribe.git
cd transcribe
powershell -ExecutionPolicy Bypass -File scripts/install.ps1
```

CPU only: add `-CpuOnly`.

Installs to `%LOCALAPPDATA%\Programs\episode-transcribe\` and adds it to your **user** `PATH`. Open a **new** terminal:

```text
transcribe --version
```

### Linux / macOS

```bash
chmod +x scripts/install.sh scripts/uninstall.sh
./scripts/install.sh          # or --cpu-only
```

Binary: `~/.local/bin/transcribe`

### Uninstall

```bash
transcribe uninstall -y                 # remove CLI + install folder
transcribe uninstall --purge -y         # also delete ~/.transcribe registry
```

Or use `scripts/uninstall.ps1` / `scripts/uninstall.sh` (works if the binary is already gone).

## Model setup

Download weights into **`transcribe/models/`** (see [models/README.md](models/README.md)). Shared across campaigns — not stored next to MP4s.

Override: `--models-dir /path` or `TRANSCRIBE_MODELS_DIR=/path`.

## Quick start

```bash
cd /path/to/my-campaign
transcribe project init my-campaign
transcribe profiles build --sample-minutes 18
transcribe profiles label
transcribe run .
```

## Commands

| Command | Description |
|---------|-------------|
| `transcribe help [TOPIC]` | Detailed reference (`project`, `profiles`, `run`, `models`, `install`) |
| `transcribe` / `transcribe run [PATH]` | Transcribe MP4s (default `PATH` = `.`) |
| `transcribe project init NAME` | Register cwd as a project |
| `transcribe project list` | List projects |
| `transcribe project use NAME` | Set active project |
| `transcribe project show` | Active project status |
| `transcribe profiles build [PATH]` | Build speaker profiles |
| `transcribe profiles label` | Interactive naming + clip playback |
| `transcribe profiles list` | List profiles |
| `transcribe profiles review` | Regenerate `voices/review.html` |
| `transcribe uninstall [-y] [--purge]` | Remove installed CLI and optional registry |

Registry: `~/.transcribe/projects.db` and `~/.transcribe/config.json`.

## Testing

```bash
# Smoke tests (no models; uses bundled test/sample.mp4)
cargo test --locked --no-default-features

# Format and lint (same checks as CI)
cargo fmt --all -- --check
cargo clippy --locked --no-default-features -- -D warnings

# Full e2e (needs models/ggml-base.bin + ffmpeg)
TRANSCRIBE_E2E=1 cargo test --release --no-default-features -- --ignored
```

## Project layout

```text
transcribe/                    # repository root
  src/
  models/                      # shared model weights (gitignored)
  test/                        # sample.mp4 (bundled speech meme fixture)
  scripts/
    install.ps1 / install.sh
    uninstall.ps1 / uninstall.sh
    build-cuda.ps1             # Windows GPU build helper
  tests/                       # integration tests

your-campaign/
  Episode 01.mp4
  Episode 01.srt
  voices/
```

## Development

```bash
cargo build --locked --no-default-features
cargo test --locked --no-default-features
cargo run --no-default-features -- help
```

CI runs on push and pull requests to `main` / `master` (see [.github/workflows/ci.yml](.github/workflows/ci.yml)): `cargo fmt`, `cargo clippy`, smoke tests, coverage, and optional e2e on `main`.

## Releases

Pre-built **CPU** binaries (`--no-default-features`, no CUDA) for Linux, Windows, and macOS. Each archive includes `transcribe` (or `transcribe.exe` on Windows) plus bundled sherpa-onnx shared libraries (`.so`, `.dll`, or `.dylib`).

| Trigger | Workflow | Output |
|---------|----------|--------|
| Push to `main` / `master` | [.github/workflows/build-artifacts.yml](.github/workflows/build-artifacts.yml) | [Actions artifacts](https://docs.github.com/en/actions/managing-workflow-runs/downloading-workflow-artifacts) (90-day retention), named `transcribe-{target}-{short-sha}` |
| Push tag `v*` | [.github/workflows/release.yml](.github/workflows/release.yml) | GitHub Release with attached assets |

**Branch pushes** package as `transcribe-v{7-char-sha}-{target}` (e.g. `transcribe-v0c8ad05-x86_64-unknown-linux-gnu.tar.gz`). Download from **Actions → Build artifacts → latest run**.

**Tagged releases** use a semver tag:

| Tag pattern | GitHub release |
|-------------|----------------|
| `v0.1.0-alpha` (ends with `-alpha`) | Pre-release |
| `v1.0.0` | Stable release |

**Publish an alpha:**

```bash
git tag v0.1.0-alpha
git push origin v0.1.0-alpha
```

**Publish a stable release:**

```bash
git tag v1.0.0
git push origin v1.0.0
```

To rebuild assets for an existing tag (e.g. a release created without artifacts), delete and re-push the tag, or run **Actions → Release → Run workflow** and enter the tag name.

Asset names follow `transcribe-v{VERSION}-{TARGET}` (version is the tag without the leading `v`), for example:

- `transcribe-v0.1.0-alpha-x86_64-unknown-linux-gnu.tar.gz`
- `transcribe-v0.1.0-alpha-x86_64-pc-windows-msvc.zip`
- `transcribe-v0.1.0-alpha-aarch64-apple-darwin.tar.gz`

Extract the archive and run `transcribe` from the folder. You still need [ffmpeg](https://ffmpeg.org/) on `PATH` and model weights — see [models/README.md](models/README.md).

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, git workflow, and review expectations. Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

MIT — see [LICENSE](LICENSE).
