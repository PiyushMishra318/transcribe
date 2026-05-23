#!/usr/bin/env bash
# Download model weights used by CI coverage tests (matches models/README.md).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

mkdir -p models

curl -fsSL -o models/ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

curl -fsSL -o models/wespeaker_en_voxceleb_resnet34_LM.onnx \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx

PYANNOTE_ARCHIVE="/tmp/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2"
curl -fsSL -o "${PYANNOTE_ARCHIVE}" \
  https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2
tar -xjf "${PYANNOTE_ARCHIVE}" -C models
rm -f "${PYANNOTE_ARCHIVE}"

test -f models/sherpa-onnx-pyannote-segmentation-3-0/model.onnx
echo "CI test models ready under ${ROOT}/models"
