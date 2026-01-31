---
created: ${timestamp}
title: Allow force stop during restart
area: tooling
files:
  - TBD
---

## Problem

Restart can hang if the service does not stop cleanly, and there is no clear way for users to force-stop during a restart.

## Solution

Add a restart option to force stop (e.g., --force) and thread it through the restart flow.
