#!/usr/bin/env pwsh

# Parse arguments
$Profile = "release"
$TargetDir = "release"

if ($args.Count -gt 0) {
    switch ($args[0]) {
        "debug" {
            $Profile = "dev"
            $TargetDir = "debug"
        }
        "release" {
            $Profile = "release"
            $TargetDir = "release"
        }
        default {
            Write-Host "usage: build.ps1 [debug|release]"
            exit 1
        }
    }
}

# Set error action preference to stop on errors
$ErrorActionPreference = "Stop"

# Emscripten pthread need atomics and bulk-memory features target features.
# all other emscripten linker flags are specified in `build.rs`
# `panic=abort` helps trimming down the generated code in size. With abort,
# errors are actually also better traceable in debug builds...
$env:RUSTFLAGS = "-Ctarget-feature=+atomics,+bulk-memory -Cpanic=abort"

# Use build-std to also compile the std libs with the above rust flags for pthreads support.
cargo +nightly -Z build-std=std,panic_abort build --profile $Profile --target wasm32-unknown-emscripten
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo build failed"
    exit $LASTEXITCODE
}

# Copy build artifacts to the web folder.
# Note: Cargo copies the .js/.wasm artifacts to the profile dir, or "deps" in older versions.
$targetPath = "target\wasm32-unknown-emscripten\$TargetDir"
function Copy-Artifact($Ext) {
    $sources = @(
        "$targetPath\playground.$Ext",
        "$targetPath\deps\playground.$Ext"
    )
    foreach ($source in $sources) {
        try {
            Copy-Item $source "web\playground.$Ext" -Force -ErrorAction Stop
            return
        } catch {
            # try next
        }
    }
    Write-Error "playground.$Ext not found"
    exit 1
}

foreach ($ext in @("wasm", "js")) {
    Copy-Artifact $ext
}

# The emscripten --preload-file sidecar stays in the linker output dir.
$data = Get-ChildItem -Path "$targetPath\build\playground\*\out\playground.data" -File -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $data) {
    Write-Error "playground.data not found"
    exit 1
}
Copy-Item $data.FullName "web\playground.data" -Force
