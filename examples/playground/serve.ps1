#!/usr/bin/env pwsh

# Set error action preference to stop on errors
$ErrorActionPreference = "Stop"

# Check if simple-http-server is installed
$command = Get-Command simple-http-server -ErrorAction SilentlyContinue

if (-not $command) {
    Write-Host "*** simple-http-server could not be found. you can install it via:"
    Write-Host "cargo binstall simple-http-server"
    exit 1
}

# Note: worker threads (emscripten pthreads) need 
# "Cross-Origin-Embedder-Policy" HTTP header set to "require-corp"
# "Cross-Origin-Opener-Policy" HTTP header set to "same-origin"
simple-http-server --index --coep --coop web
