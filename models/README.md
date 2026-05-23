# Model files

Download weights **here** (inside the `transcribe` repo). The CLI does not use a `models/` folder in your campaign directory unless you override with `--models-dir`.

Default lookup order:

1. `TRANSCRIBE_MODELS_DIR` environment variable  
2. `models/` next to the installed `transcribe` binary  
3. `transcribe/models/` when your campaign folder sits beside this repo  
4. `~/.transcribe/models/` as a last resort  

Files are **not** shipped with the repository because of size.

## Required — Whisper

| File | Purpose | Download |
|------|---------|----------|
| `ggml-medium.bin` | Recommended quality (~1.5 GB) | [whisper.cpp on Hugging Face](https://huggingface.co/ggerganov/whisper.cpp/tree/main) |
| `ggml-base.bin` | Faster, lower quality (~150 MB) | same |

Use `--model base` or `--model medium` when running `transcribe run`.

## Optional — voice activity detection

| File | Download |
|------|----------|
| `ggml-silero-v6.2.0.bin` | [whisper-vad](https://huggingface.co/ggml-org/whisper-vad) |

Auto-enabled when present.

## Required for speaker tagging (`profiles build`)

| File | Download |
|------|----------|
| `sherpa-onnx-pyannote-segmentation-3-0/model.onnx` | [segmentation tarball](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2) |
| `wespeaker_en_voxceleb_resnet34_LM.onnx` | [embedding model](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx) |

Extract the segmentation archive so `model.onnx` lives under `models/sherpa-onnx-pyannote-segmentation-3-0/`.
