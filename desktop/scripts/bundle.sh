#!/usr/bin/env bash
# Build the installers: the `seesea` binary goes in as a sidecar next to the shell.
# Needs `cargo install tauri-cli --locked` once.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
triple="$(rustc -vV | sed -n 's/^host: //p')"
ext=""; case "$triple" in *windows*) ext=".exe";; esac
cargo build --release --manifest-path "$here/../seesea-core/Cargo.toml" -p seesea-cli
mkdir -p "$here/binaries"
cp "$here/../seesea-core/target/release/seesea$ext" "$here/binaries/seesea-$triple$ext"
cd "$here"
cargo tauri build --config tauri.bundle.conf.json "$@"
