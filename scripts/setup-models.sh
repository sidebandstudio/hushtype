#!/bin/bash
# Model setup runs outside hushtype, whose runtime only calls configured endpoints.
# Every file comes from its original publisher at a pinned revision and must
# match a known SHA-256 before it is installed.
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
model_only=false
if [ "${1:-}" = --model-only ]; then
  model_only=true
  shift
fi

# fetch <url> <sha256> <destination>
fetch() {
  curl --fail --location --retry 3 --retry-all-errors --output "$3.download" "$1"
  printf '%s  %s\n' "$2" "$3.download" | shasum -a 256 -c -
  mv "$3.download" "$3"
}

# Silero VAD v4, from the Silero project's v4.0 release (MIT).
vad_file="$repo_dir/src-tauri/resources/models/silero_vad_v4.onnx"
vad_sha="a35ebf52fd3ce5f1469b2a36158dba761bc47b973ea3382b3186ca15b1f5af28"
if [ "$model_only" = false ] && ! printf '%s  %s\n' "$vad_sha" "$vad_file" | shasum -a 256 -c - >/dev/null 2>&1; then
  mkdir -p "$(dirname "$vad_file")"
  fetch https://raw.githubusercontent.com/snakers4/silero-vad/915dd3d639b8333a52e001af095f87c5b7f1e0ac/files/silero_vad.onnx \
    "$vad_sha" "$vad_file"
fi

# NVIDIA Parakeet TDT 0.6B v3 (CC BY 4.0), int8 ONNX conversion published on
# Hugging Face by istupakov, pinned to one repository revision.
model_dir="${1:-$HOME/Library/Application Support/dev.hushtype.app/models}"
model_name="parakeet-tdt-0.6b-v3-int8"
hf_base="https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce"
model_files="
config.json 666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466
decoder_joint-model.int8.onnx eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70
encoder-model.int8.onnx 6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09
nemo128.onnx a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f
vocab.txt d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d
"

if [ -d "$model_dir/$model_name" ]; then
  while read -r file sha; do
    [ -n "$file" ] || continue
    if [ ! -s "$model_dir/$model_name/$file" ]; then
      printf 'Existing model folder is incomplete: %s\nMove it aside and run this installer again.\n' "$model_dir/$model_name" >&2
      exit 1
    fi
  done <<< "$model_files"
  printf 'Parakeet V3 is already installed at %s\n' "$model_dir/$model_name"
  exit 0
fi

mkdir -p "$model_dir"
# Stage beside the destination so the final rename is atomic: the app never
# sees a partially downloaded model folder.
stage_dir="$(mktemp -d "$model_dir/.$model_name.XXXXXX")"
trap 'rm -rf "$stage_dir"' EXIT
while read -r file sha; do
  [ -n "$file" ] || continue
  fetch "$hf_base/$file" "$sha" "$stage_dir/$file"
done <<< "$model_files"
mv "$stage_dir" "$model_dir/$model_name"
printf 'Installed %s. Open hushtype and refresh the model list.\n' "$model_dir/$model_name"
