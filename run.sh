#!/bin/sh
set -eu
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PATH"
case "${1:-}" in
  "") flowmoor_profile=debug; flowmoor_build_flag= ;;
  --release) flowmoor_profile=release; flowmoor_build_flag=--release ;;
  *) echo "Usage: ./run.sh [--release]" >&2; exit 2 ;;
esac
cargo fetch --locked
python3 scripts/generate-third-party-notices.py
if [ -z "${FLOWMOOR_YTDLP:-}" ]; then
  for tool in "$PWD/.tools/yt-dlp-macos/yt-dlp" "$PWD/.tools/yt-dlp"; do
    if [ -x "$tool" ]; then export FLOWMOOR_YTDLP="$tool"; break; fi
  done
fi
if [ "$(uname -s)" = "Darwin" ]; then
  cargo build --locked $flowmoor_build_flag --bin flowmoor-rust
  flowmoor_bundle="$PWD/target/Flowmoor Rust.app"
  mkdir -p "$flowmoor_bundle/Contents/MacOS" "$flowmoor_bundle/Contents/Resources"
  cp assets/macos/Info.plist "$flowmoor_bundle/Contents/Info.plist"
  cp assets/macos/launch "$flowmoor_bundle/Contents/MacOS/launch"
  cp "target/$flowmoor_profile/flowmoor-rust" "$flowmoor_bundle/Contents/MacOS/flowmoor-rust"
  cp third-party/THIRD_PARTY_NOTICES.html "$flowmoor_bundle/Contents/Resources/THIRD_PARTY_NOTICES.html"
  chmod +x "$flowmoor_bundle/Contents/MacOS/launch"
  exec "$flowmoor_bundle/Contents/MacOS/launch"
fi
exec cargo run --locked $flowmoor_build_flag --bin flowmoor-rust
