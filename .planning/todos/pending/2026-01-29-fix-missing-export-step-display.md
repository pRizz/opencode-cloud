---
created: 2026-01-29T15:23
title: Fix missing export step display
area: tooling
files:
  - packages/core/src/docker/image.rs:360-460
---

## Problem

Docker build progress output can miss the final "exporting to image" step or
display a stale message from a previous step. We need to ensure the export step
is surfaced as the newest build step when it appears.

## Solution

Investigate BuildKit log/vertex sequencing and confirm the best source of truth
for the export step (logs vs vertex names). Update the progress selection logic
to prioritize the export step reliably.
