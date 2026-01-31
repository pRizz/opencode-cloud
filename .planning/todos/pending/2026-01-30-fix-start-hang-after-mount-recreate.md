---
created: 2026-01-30T21:51
title: Fix start hang after mount recreate
area: tooling
files:
  - packages/cli-rust/src/commands/service.rs:20
  - packages/cli-rust/src/commands/start.rs:700
---

## Problem

When running `just run start` and mounts change, confirming the recreate prompt stops the
container (eventually force-killed after the 30s timeout) and starts the service, but the
process remains open until the user presses Enter.

## Solution

Ensure the force-kill prompt does not keep stdin reads alive after the stop completes.
Cancel or drop pending input tasks after `select!` resolves so the command can exit cleanly.
