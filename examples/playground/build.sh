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
# all other emscripten linker flags are specified in `build.rs`` 
export RUSTFLAGS="-Ctarget-feature=+atomics,+bulk-memory"

# Use build-std to also compile the std libs with the above rust flags for pthreads support.
cargo +nightly -Z build-std \
  build --profile $PROFILE --target wasm32-unknown-emscripten

## copy

# Copy build artifacts to the web folder.
for ext in wasm js data; do
  cp "target/wasm32-unknown-emscripten/$TARGET_DIR/deps/playground.$ext" ./web
done