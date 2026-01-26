---
phase: 18-cli-sync-strategy
plan: 05
subsystem: cli-node
tags: [optionalDependencies, binary-resolution, platform-detection, musl]
requires:
  - 18-04
provides:
  - optionalDependencies for all six platform packages
  - Platform-aware binary resolution (darwin/linux, glibc/musl)
  - Local bin/ fallback for development
  - Updated README for prebuilt distribution
affects:
  - 18-06 # Publish workflow
key-files:
  modified:
    - packages/cli-node/package.json
    - packages/cli-node/src/index.ts
    - packages/cli-node/README.md
decisions:
  - name: "workspace:* for optionalDependencies in dev"
    rationale: "Resolve to local platform packages; pnpm publish rewrites to version when publishing"
  - name: "isMusl via /etc/alpine-release and ldd --version"
    rationale: "Reliable Alpine/musl detection; plan's ldd binary read was incorrect"
  - name: "Fallback: platform package first, then local bin/occ"
    rationale: "Dev and CI use local binary; published installs use platform package"
metrics:
  duration: "~10 min"
  completed: "2026-01-25"
---

# Phase 18 Plan 05: optionalDependencies + Binary Resolution Summary

**One-liner:** cli-node uses optionalDependencies for platform packages, resolves binary via require(platformPkg).binaryPath with local bin/ fallback; README updated.

## What Was Built

- **package.json:** optionalDependencies (workspace:*) for all six platform packages; description updated to "Cross-platform CLI (includes prebuilt binaries)".
- **index.ts:** `getPlatformPackage()` (darwin/linux, arch, musl); `resolveBinaryPath()` tries platform package then `../bin/occ`; `isMusl()` via `/etc/alpine-release` and `ldd --version`; clear errors for missing binary or unsupported platform.
- **README.md:** Installation (npm/npx, no Rust), Supported Platforms table, How it works, Development (local bin/).

## Verification

- `pnpm -C packages/cli-node build` succeeds
- With `cp target/debug/occ packages/cli-node/bin/occ`, `node packages/cli-node/dist/index.js --version` works (fallback)
- `just run --version` works (cargo run)
- `just test` passes (including cli-parity)

## Next

18-06: build-cli-binaries workflow, pnpm workspace updates, publish-npm workflow.
