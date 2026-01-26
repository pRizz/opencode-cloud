---
phase: 18-cli-sync-strategy
plan: 04
subsystem: npm
tags: [platform-packages, optionalDependencies, prebuilt, esbuild-pattern]
requires: []
provides:
  - Six platform-specific npm package scaffolds
  - os/cpu/libc fields for npm auto-selection
  - binaryPath export for main package resolution
affects:
  - 18-05 # optionalDependencies + resolution
  - 18-06 # CI build + publish
key-files:
  created:
    - packages/cli-darwin-arm64/package.json
    - packages/cli-darwin-arm64/index.js
    - packages/cli-darwin-arm64/bin/.gitkeep
    - packages/cli-darwin-x64/package.json
    - packages/cli-darwin-x64/index.js
    - packages/cli-darwin-x64/bin/.gitkeep
    - packages/cli-linux-x64/package.json
    - packages/cli-linux-x64/index.js
    - packages/cli-linux-x64/bin/.gitkeep
    - packages/cli-linux-arm64/package.json
    - packages/cli-linux-arm64/index.js
    - packages/cli-linux-arm64/bin/.gitkeep
    - packages/cli-linux-x64-musl/package.json
    - packages/cli-linux-x64-musl/index.js
    - packages/cli-linux-x64-musl/bin/.gitkeep
    - packages/cli-linux-arm64-musl/package.json
    - packages/cli-linux-arm64-musl/index.js
    - packages/cli-linux-arm64-musl/bin/.gitkeep
decisions:
  - name: "optionalDependencies pattern (esbuild/swc)"
    rationale: "npm installs only matching platform package; minimal footprint"
  - name: "Six platforms: darwin arm64/x64, linux x64/arm64 glibc, linux x64/arm64 musl"
    rationale: "Cover macOS, common Linux, and Alpine/musl"
metrics:
  duration: "~5 min"
  completed: "2026-01-25"
---

# Phase 18 Plan 04: Platform-Specific npm Package Scaffolds Summary

**One-liner:** Six platform package directories with package.json, index.js (binaryPath), and bin/.gitkeep, ready for CI binary insertion and npm publish.

## What Was Built

- **Darwin:** `cli-darwin-arm64`, `cli-darwin-x64` (os: darwin, cpu: arm64|x64)
- **Linux glibc:** `cli-linux-x64`, `cli-linux-arm64` (os: linux, libc: glibc)
- **Linux musl:** `cli-linux-x64-musl`, `cli-linux-arm64-musl` (os: linux, libc: musl)

Each package has `package.json` (name, version 3.0.0, os, cpu, libc, files, main), `index.js` exporting `binaryPath`, and `bin/.gitkeep` as placeholder for the `occ` binary.

## Verification

- All six directories exist under `packages/`
- Each has package.json with correct name, os, cpu, and (where applicable) libc
- Each has index.js exporting `binaryPath` pointing to `bin/occ`
- Each has `bin/.gitkeep`

## Next

18-05 adds optionalDependencies to cli-node and platform-aware binary resolution.
