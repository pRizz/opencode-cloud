---
created: 2026-01-27T11:50
title: Investigate hanging graceful stop
area: tooling
files:
  - packages/cli-rust/src/commands/start.rs:299
  - packages/cli-rust/src/commands/stop.rs:49
  - packages/cli-rust/src/commands/service.rs:11
---

## Problem

When running `just run start --cached-rebuild-sandbox-image` with a running container, confirming the stop prompt appears to hang before the image build begins. We expected visible progress for the graceful stop step but it doesn't appear, suggesting a regression in the stop flow or output.

## Solution

Trace the stop path triggered during `start` rebuilds and ensure progress is shown (spinner or logs) while waiting for Docker to stop the container. Verify `stop_service` behavior, timeout handling, and output suppression, then adjust as needed.
