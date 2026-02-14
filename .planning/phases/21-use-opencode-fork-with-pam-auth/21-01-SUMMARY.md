---
phase: 21-use-opencode-fork-with-pam-auth
plan: 01
subsystem: dockerfile
tags: [opencode-fork, pam-auth, opencode-broker, systemd]
requires: []
provides:
  - opencode fork installation (pinned to commit 11277fd04fb7f6df4b6f188397dcb5275ef3a78f)
  - opencode-broker binary build and installation
  - PAM configuration file (/etc/pam.d/opencode)
  - opencode-broker.service systemd unit
  - opencode.service updated with broker dependency
  - opencode.json config with auth enabled
affects:
  - Docker image builds (now uses fork instead of official installer)
  - Authentication flow (PAM-based instead of basic auth)
key-files:
  modified:
    - packages/core/src/docker/Dockerfile
decisions:
  - name: "Install bun during opencode build"
    rationale: "Fork requires bun for building; install it inline during the build process"
  - name: "Build broker as root for setuid permissions"
    rationale: "Broker needs setuid (4755) to access PAM; must be installed as root"
  - name: "Pin to specific commit for reproducibility"
    rationale: "Commit 11277fd04fb7f6df4b6f188397dcb5275ef3a78f ensures consistent builds"
  - name: "Create opencode.json config during build"
    rationale: "Auth must be enabled in config file; create it during image build"
metrics:
  duration: "~15 min"
  completed: "2026-01-26"
---

# Phase 21 Plan 01: Dockerfile Update and PAM Integration Summary

**One-liner:** Dockerfile updated to install opencode from pRizz/opencode fork, build opencode-broker, install PAM config, and set up systemd services for PAM-based authentication.

## What Was Built

- **opencode fork installation:** Replaced official installer script with git clone + build from source, pinned to commit `11277fd04fb7f6df4b6f188397dcb5275ef3a78f`. Installs bun inline during build, uses `bun run packages/opencode/script/build.ts --single` to build platform-specific binary.

- **opencode-broker build:** Added section to clone fork, build Rust broker binary from `packages/opencode-broker`, install to `/usr/local/bin/opencode-broker` with setuid permissions (4755) for PAM access.

- **PAM configuration:** Created `/etc/pam.d/opencode` with standard UNIX authentication (`pam_unix.so`) for both auth and account modules. Includes commented 2FA option for future use.

- **opencode-broker.service:** Created systemd unit file with Type=notify, security hardening settings, RuntimeDirectory for socket creation, and enabled via symlink.

- **opencode.service update:** Added `After=opencode-broker.service` dependency to ensure broker starts before opencode and creates the Unix socket.

- **opencode.json config:** Created `/home/opencode/.config/opencode/opencode.json` with minimal config `{"auth": {"enabled": true}}` to enable PAM authentication.

## Verification

- Dockerfile syntax validated (no errors)
- All sections in correct order
- Commit hash verified: `11277fd04fb7f6df4b6f188397dcb5275ef3a78f`
- Service dependencies correct (opencode After=opencode-broker.service)
- File permissions appropriate (setuid for broker, 644 for configs)
- All temporary build directories cleaned up

## Notes

- Bun is installed inline during opencode build (not pre-installed in Dockerfile)
- Broker build runs as root to set setuid permissions, then switches back to opencode user
- PAM config uses standard UNIX authentication; 2FA support is commented for future use
- opencode.json config is created during build to ensure auth is enabled by default
- Both services are enabled via symlinks (systemctl doesn't work during Docker build)
