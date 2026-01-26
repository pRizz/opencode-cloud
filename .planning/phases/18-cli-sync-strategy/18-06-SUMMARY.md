---
phase: 18-cli-sync-strategy
plan: 06
subsystem: ci
tags: [github-actions, cross-compilation, npm-publish, workflow_call]
requires:
  - 18-04
  - 18-05
provides:
  - build-cli-binaries workflow (six-target matrix, cross for ARM)
  - publish-npm workflow (build → download artifacts → publish platform then main)
  - pnpm workspace explicit list including platform packages
affects:
  - version-bump (when publish-npm uncommented)
key-files:
  created:
    - .github/workflows/build-cli-binaries.yml
  modified:
    - .github/workflows/publish-npm.yml
    - pnpm-workspace.yaml
decisions:
  - name: "cross for ARM Linux targets"
    rationale: "No native ARM runners; cross uses Docker for cross-compilation"
  - name: "workflow_call + artifacts in same run"
    rationale: "build-cli-binaries runs as reusable workflow; artifacts available to publish job"
  - name: "ref input on publish-npm for version-bump"
    rationale: "Checkout release tag when called from version-bump; matches publish-crates/docker-publish"
  - name: "Build --bin occ explicitly"
    rationale: "Package has two binaries; we only need occ for platform packages"
metrics:
  duration: "~10 min"
  completed: "2026-01-25"
---

# Phase 18 Plan 06: CI Build + Publish Summary

**One-liner:** build-cli-binaries workflow builds Rust CLI for all six platforms; publish-npm calls it, downloads artifacts, publishes platform packages then main; pnpm workspace lists all packages.

## What Was Built

- **build-cli-binaries.yml:** workflow_call + workflow_dispatch. Matrix: darwin arm64/x64 (macos), linux x64/arm64 glibc, linux x64/arm64 musl. Uses `cross` for ARM targets, musl-tools for musl x64. Builds `-p opencode-cloud --bin occ`, copies to `packages/<pkg>/bin/occ`, uploads per-package artifacts (retention 1 day).
- **publish-npm.yml:** build-binaries job (uses build-cli-binaries), publish job (checkout, install, download all six artifacts, chmod +x, build core + cli-node, publish platform packages, sleep 30, publish core, sleep 10, publish opencode-cloud, summary). Inputs: version (required), ref (optional). Permissions: contents read, id-token write.
- **pnpm-workspace.yaml:** Explicit package list including all six platform packages.

## Verification

- `just fmt && just lint && just build && just test` pass.
- Build-cli-binaries can be run manually via workflow_dispatch for local validation.

## Notes

- version-bump still has publish-npm commented out. When enabled, pass `ref: ${{ needs.bump-version.outputs.ref }}` and `secrets: inherit`.
- Platform package optionalDependencies use `workspace:*`; pnpm publish rewrites to concrete versions on publish.
