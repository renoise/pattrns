#!/bin/bash

set -e -o pipefail

# check if simple-http-server is installed
if ! command -v simple-http-server &> /dev/null; then
  echo '*** simple-http-server could not be found. you can install it via:'
  echo 'cargo binstall simple-http-server'
  exit 1
fi

# Note: worker threads (emscripten pthreads) need 
# "Cross-Origin-Embedder-Policy" HTTP header set to "require-corp"
# "Cross-Origin-Opener-Policy" HTTP header set to "same-origin"
simple-http-server --index --coep --coop  web
