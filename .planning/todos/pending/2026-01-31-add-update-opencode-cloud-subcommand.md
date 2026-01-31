---
created: 2026-01-31T13:01
title: Add update subcommand for opencode-cloud
area: tooling
files:
  - TBD
---

## Problem

There is no CLI command to update the opencode-cloud binary itself and restart the service after reinstall, which makes self-upgrades error-prone.

## Solution

Add an update subcommand that reinstalls opencode-cloud (via cargo or npm as appropriate) and restarts the service after the update completes.
