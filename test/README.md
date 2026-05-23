# Test fixtures

## `sample.mp4`

Short clip from **Rick Astley — “Never Gonna Give You Up”** (classic Rickroll, chorus ~43–56 s, ~13 s). Bundled in the repo for CI and local tests — no download step.

| File | Purpose |
|------|---------|
| `sample.mp4` | Input video |
| `sample.txt` | Golden plain transcript (Whisper `base`) |
| `sample.srt` | Golden subtitles (same run) |

Regenerate after changing the clip:

```bash
cd test
../target/release/transcribe run . --model base --models-dir ../models --no-speakers --force
```

**Expected transcript phrases** (used by e2e; Whisper may vary slightly):

- `never`
- `gonna`
- `give`
- `let`
- `down`

## Running tests

```bash
cargo test --no-default-features
TRANSCRIBE_E2E=1 cargo test --release -- --ignored   # needs models/ggml-base.bin
```
