---
created: 2026-01-31T12:59
title: Add update-opencode subcommand
area: tooling
files:
  - TBD
---

## Problem

There is no CLI subcommand to update the opencode runtime inside the container without rebuilding the entire image. Users need a guided way to stop the service, update the repo, rebuild, and restart.

## Solution

Add an update-opencode subcommand that stops the inner opencode service, pulls/clones https://github.com/pRizz/opencode.git, runs the same build steps as the Dockerfile, and restarts the opencode service.
