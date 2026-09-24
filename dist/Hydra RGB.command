#!/bin/bash
set -euo pipefail
if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
if [[ -f "$HERE/Cargo.toml" ]]; then
  cd "$HERE"
elif [[ -f "$HERE/../hydra-rs/Cargo.toml" ]]; then
  cd "$HERE/../hydra-rs"
else
  echo "Keep this file in the hydra folder, next to the hydra-rs directory."
  read -r -p "Press Return to close."
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Install Rust from https://rustup.rs, then double-click this file again."
  read -r -p "Press Return to close."
  exit 1
fi

if ! xcode-select -p >/dev/null 2>&1; then
  echo "macOS command line tools are required. A setup window should open."
  xcode-select --install || true
  echo "When that install finishes, double-click this file again."
  read -r -p "Press Return to close."
  exit 1
fi

echo "Building Hydra RGB for this Mac..."
cargo build --release
echo "Opening Hydra RGB."
exec ./target/release/hydra-rgb