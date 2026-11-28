#!/bin/bash

set -e -o pipefail

## options

usage() {
  echo "usage: `basename $0` [debug|release]"
}

PROFILE="release"
TARGET_DIR="release"
while [ "$1" ]; do
  if [ "$1" = "debug" ]; then
    PROFILE="dev"
    TARGET_DIR="debug"
    shift
  elif [ "$1" = "release" ]; then
    PROFILE="release"
    TARGET_DIR="release"
    shift
  else
    usage; exit 1
  fi
done

## build

# Emscripten pthread need atomics and bulk-memory features target features.
# all other emscripten linker flags are specified in `build.rs`
# `panic=abort` helps trimming down the generated code in size. With abort,
# errors are actually also better traceable in debug builds...
export RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory -Cpanic=abort"

# Use build-std to also compile the std libs with the above rust flags for pthreads support.
cargo +nightly -Z build-std=std,panic_abort \
  build --profile $PROFILE --target wasm32-unknown-emscripten

## copy

# Copy build artifacts to the web folder.
# Note: Cargo copies the .js/.wasm artifacts to the profile dir, or "deps" in older versions.
target_dir="target/wasm32-unknown-emscripten/$TARGET_DIR"
out_dir="$(ls -td "$target_dir/build/playground"/*/out 2>/dev/null | sed -n '1p')" || true
if [ -z "$out_dir" ]; then
  echo "error: emscripten build output dir not found" >&2
  exit 1
fi

for ext in wasm js; do
  cp "$target_dir/playground.$ext" ./web 2>/dev/null ||
  cp "$target_dir/deps/playground.$ext" ./web
done

cp "$out_dir/playground.data" ./web
