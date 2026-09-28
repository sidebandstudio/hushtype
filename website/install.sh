#!/bin/bash
# hushtype installer for Macs with Apple silicon.
#
#   curl -fsSL https://hushtype.vercel.app/install.sh | bash
#
# Downloads hushtype and its speech model, checks every file against a known
# SHA-256, installs the app into Applications, and opens it. Running it again
# updates the app and keeps your settings and model.
#
# Everything runs inside main(), so a partial download of this script does nothing.

set -euo pipefail

VERSION="0.3.0"
APP_URL="https://hushtype.vercel.app/download/hushtype-0.3.0-mac.zip"
APP_SHA256="b4f0b93bd3dd6979f138142e626f3dc9037fdba91c70947bb9e4f49f6e71fe03"
APP_SIZE="18 MB"

# NVIDIA Parakeet TDT 0.6B v3 (CC BY 4.0), int8 ONNX conversion on Hugging Face,
# pinned to one repository revision.
MODEL_NAME="parakeet-tdt-0.6b-v3-int8"
MODEL_BASE="https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce"
MODEL_FILES="
config.json 666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466
decoder_joint-model.int8.onnx eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70
encoder-model.int8.onnx 6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09
nemo128.onnx a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f
vocab.txt d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d
"
MODEL_DIR="$HOME/Library/Application Support/dev.hushtype.app/models"

# Temporary download locations, removed when the script exits for any reason.
WORK=""
STAGE=""
cleanup() {
  if [ -n "$WORK" ]; then rm -rf "$WORK"; fi
  if [ -n "$STAGE" ]; then rm -rf "$STAGE"; fi
}

if [ -t 1 ]; then
  BOLD=$'\033[1m' DIM=$'\033[2m' GREEN=$'\033[32m' RED=$'\033[31m' RESET=$'\033[0m'
else
  BOLD="" DIM="" GREEN="" RED="" RESET=""
fi

step() { printf '\n%s%s%s\n' "$BOLD" "$1" "$RESET"; }
note() { printf '%s%s%s\n' "$DIM" "$1" "$RESET"; }
fail() {
  printf '\n%sInstall stopped.%s %s\n' "$RED$BOLD" "$RESET" "$1" >&2
  exit 1
}

check_mac() {
  if [ "$(uname -s)" != "Darwin" ]; then
    fail "hushtype runs on Macs with Apple silicon. Windows support is coming soon."
  fi
  # hw.optional.arm64 is 1 on Apple silicon, even in a Terminal running under Rosetta.
  if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" != "1" ]; then
    fail "hushtype needs a Mac with Apple silicon (M1 or later). This Mac has an Intel processor."
  fi
  local macos
  macos="$(sw_vers -productVersion)"
  if [ "${macos%%.*}" -lt 14 ]; then
    fail "hushtype needs macOS 14 Sonoma or later. This Mac runs macOS $macos. Update macOS in System Settings, then run the command again."
  fi
  local free_kb
  free_kb="$(df -Pk "$HOME" | awk 'NR == 2 { print $4 }')"
  if [ "$free_kb" -lt 1500000 ]; then
    fail "hushtype needs about 1.5 GB of free disk space while it installs. Free up some space, then run the command again."
  fi
}

# fetch <url> <sha256> <destination> <what>
fetch() {
  if ! curl --fail --location --retry 3 --retry-all-errors --progress-bar --output "$3.part" "$1"; then
    rm -f "$3.part"
    fail "Could not download $4. Check your internet connection, then run the command again."
  fi
  if ! printf '%s  %s\n' "$2" "$3.part" | shasum -a 256 -c --status -; then
    rm -f "$3.part"
    fail "$4 did not match its expected checksum, so nothing was installed. Run the command again."
  fi
  mv "$3.part" "$3"
}

install_app() {
  step "1 of 3   Downloading hushtype $VERSION ($APP_SIZE)"
  WORK="$(mktemp -d)"
  fetch "$APP_URL" "$APP_SHA256" "$WORK/hushtype.zip" "hushtype"
  ditto -x -k "$WORK/hushtype.zip" "$WORK/app"
  [ -d "$WORK/app/hushtype.app" ] || fail "The download did not contain hushtype.app. Run the command again."

  if [ -w /Applications ]; then
    APP_DIR="/Applications"
  else
    APP_DIR="$HOME/Applications"
    mkdir -p "$APP_DIR"
  fi

  UPDATING=false
  if pgrep -x hushtype >/dev/null; then
    note "Closing the running copy of hushtype..."
    pkill -x hushtype || true
    for _ in 1 2 3 4 5 6 7 8 9 10; do
      pgrep -x hushtype >/dev/null || break
      sleep 0.5
    done
    pkill -9 -x hushtype 2>/dev/null || true
  fi
  for existing in "/Applications/hushtype.app" "$HOME/Applications/hushtype.app"; do
    if [ -d "$existing" ]; then
      UPDATING=true
      rm -rf "$existing"
    fi
  done

  mv "$WORK/app/hushtype.app" "$APP_DIR/hushtype.app"
  xattr -dr com.apple.quarantine "$APP_DIR/hushtype.app" 2>/dev/null || true
  if [ "$UPDATING" = true ]; then
    # macOS ties Accessibility access to the exact build of an app. Clearing
    # hushtype's old entry (and only hushtype's) lets macOS ask again cleanly.
    tccutil reset Accessibility dev.hushtype.app >/dev/null 2>&1 || true
  fi
  printf '%sInstalled %s/hushtype.app%s\n' "$GREEN" "$APP_DIR" "$RESET"
}

model_installed() {
  local file sha
  [ -d "$MODEL_DIR/$MODEL_NAME" ] || return 1
  while read -r file sha; do
    [ -n "$file" ] || continue
    [ -s "$MODEL_DIR/$MODEL_NAME/$file" ] || return 1
  done <<< "$MODEL_FILES"
}

install_model() {
  step "2 of 3   Downloading the speech model (Parakeet V3, 670 MB)"
  if model_installed; then
    printf '%sAlready installed.%s\n' "$GREEN" "$RESET"
    return
  fi
  note "It runs entirely on your Mac. This is the longest step: a few minutes on a fast connection."
  mkdir -p "$MODEL_DIR"
  rm -rf "${MODEL_DIR:?}/$MODEL_NAME"
  # Stage beside the destination so the final rename is atomic.
  local file sha
  STAGE="$(mktemp -d "$MODEL_DIR/.$MODEL_NAME.XXXXXX")"
  while read -r file sha; do
    [ -n "$file" ] || continue
    fetch "$MODEL_BASE/$file" "$sha" "$STAGE/$file" "the speech model"
  done <<< "$MODEL_FILES"
  mv "$STAGE" "$MODEL_DIR/$MODEL_NAME"
  STAGE=""
  printf '%sSpeech model ready.%s\n' "$GREEN" "$RESET"
}

open_app() {
  step "3 of 3   Opening hushtype"
  open "$APP_DIR/hushtype.app"
  cat <<DONE

${GREEN}${BOLD}hushtype is installed.${RESET}

Next, in the hushtype window:
  1. Allow Microphone, so hushtype can hear you.
  2. Allow Accessibility, so hushtype can type into other apps.
     macOS opens System Settings. Turn hushtype on there, then come back.
  3. Choose Parakeet V3 as your speech model.

Then click into any text field, hold Option-Space, speak, and let go.
hushtype lives in your menu bar. To update later, run the same command again.

DONE
}

main() {
  trap cleanup EXIT
  printf '\n%shushtype installer%s\n' "$BOLD" "$RESET"
  check_mac
  install_app
  install_model
  open_app
}

main "$@"
