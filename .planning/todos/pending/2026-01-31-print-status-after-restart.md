---
created: 2026-01-31T10:55
title: Print status after restart
area: tooling
files:
  - TBD
---

## Problem

After a successful restart, users do not see the same status output that the status subcommand provides, which makes it harder to confirm runtime state.

## Solution

After restart succeeds, display the same status output as State:       exited
Container:   opencode-cloud (572be8ab7468)
Image:       prizz/opencode-cloud-sandbox:latest
CLI:         v3.1.4
Image src:   built from source
Health:      unhealthy
Config:      /Users/peterryszkiewicz/.config/opencode-cloud/config.json
Installed:   yes (starts on login)

Security
--------
Binding:     0.0.0.0 [NETWORK EXPOSED]
Auth users:  admin, myuser, myuser2, testuser, test
Trust proxy: yes
Rate limit:  5 attempts / 60s window

Last run:    2026-01-30 15:41:47 UTC

Run 'occ start' to start the service..
