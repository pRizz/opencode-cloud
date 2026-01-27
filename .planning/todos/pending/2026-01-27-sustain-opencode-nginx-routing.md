---
created: 2026-01-27T10:58
title: Sustain opencode nginx routing
area: tooling
files:
  - packages/core/src/docker/Dockerfile
---

## Problem

The container UI depends on nginx proxying backend routes. Right now the nginx
configuration uses explicit path lists that have to be kept in sync with
opencode backend endpoints, which leads to breakages when new routes appear.

## Solution

Investigate a more sustainable approach, such as content-type aware proxying,
explicit asset handling with try_files, or a backend-driven base URL for the UI.
