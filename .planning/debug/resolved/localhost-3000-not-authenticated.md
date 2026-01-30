---
status: resolved
trigger: curl localhost:3000 returns {"error":"Not authenticated"} from EC2 instance while opencode container running; expected redirect to home page.
created: 2026-01-30T00:00:00Z
updated: 2026-01-30T23:26:12Z
---

## Current Focus

hypothesis: Verified curl behavior matches Accept header handling; issue is expected behavior, and Dockerfile health check fix is correct.
test: User confirmed curl results for both default and text/html Accept headers.
expecting: Default curl returns JSON 401; Accept: text/html returns 302 redirect to /auth/login.
next_action: Archive debug session.

## Symptoms

expected: Redirect to home page (302/HTML)
actual: {"error":"Not authenticated"}
errors: User is not sure
reproduction: curl localhost:3000 (no headers)
timeline: Never worked from within EC2 instance yet

## Eliminated

## Evidence

- timestamp: 2026-01-30T00:00:00Z
  checked: Dockerfile opencode configuration
  found: Config file created at `/home/opencode/.config/opencode/opencode.jsonc` with `{"auth": {"enabled": true}}`
  implication: Authentication is enabled in opencode web server

- timestamp: 2026-01-30T00:00:00Z
  checked: Codebase for "Not authenticated" error message
  found: Error message not found in this codebase - must be coming from opencode binary (pRizz fork)
  implication: The opencode web server itself is returning the JSON error response

- timestamp: 2026-01-30T00:00:00Z
  checked: opencode web server command
  found: Runs as `opencode web --port 3000 --hostname 0.0.0.0` inside container
  implication: Server is running and should be accessible on port 3000

- timestamp: 2026-01-30T00:00:00Z
  checked: curl localhost:3000 with default Accept header
  found: Returns HTTP 401 with JSON `{"error":"Not authenticated"}` and Content-Type: application/json
  implication: Server treats request as API call when Accept: */*

- timestamp: 2026-01-30T00:00:00Z
  checked: curl localhost:3000 with Accept: text/html header
  found: Returns HTTP 302 redirect to `/auth/login` with Location header
  implication: Server correctly redirects when it detects browser-like request (Accept: text/html)

- timestamp: 2026-01-30T00:00:00Z
  checked: Docker container health status
  found: Container shows as "unhealthy" - health check is failing
  implication: Health check uses curl without Accept header, gets 401 JSON, which fails health check

- timestamp: 2026-01-30T00:00:00Z
  checked: Dockerfile health check command
  found: Uses `curl -f http://localhost:3000/` without Accept header
  implication: Health check needs to send Accept: text/html to get redirect instead of 401 JSON

- timestamp: 2026-01-30T00:00:00Z
  checked: Applied fix to Dockerfile health check
  found: Updated health check to send `Accept: text/html` header
  implication: Health check will now get 302 redirect instead of 401 JSON, allowing health check to pass

- timestamp: 2026-01-30T00:00:00Z
  checked: Verified fix works locally
  found: curl with Accept: text/html returns 302 redirect as expected
  implication: Fix is correct - health check will pass after container rebuild

- timestamp: 2026-01-30T23:13:50Z
  checked: Dockerfile health check content
  found: Health check uses `curl -f -H "Accept: text/html" http://localhost:3000/`
  implication: Image rebuild is required to apply health check change; curl without Accept header will still return JSON 401 by design

- timestamp: 2026-01-30T23:30:00Z
  checked: Dockerfile health check content
  found: Reapplied fix to use `curl -f -H "Accept: text/html" http://localhost:3000/`
  implication: Rebuild needed to pick up the health check fix

- timestamp: 2026-01-30T23:26:12Z
  checked: User verification of curl behavior
  found: curl without Accept header returns 401 JSON; curl with Accept: text/html returns 302 redirect to /auth/login
  implication: Behavior matches Accept-header-based response selection; issue is expected for non-browser curl

## Resolution

root_cause: opencode web server uses Accept header to determine response format: API clients (Accept: */*) get JSON 401, browsers (Accept: text/html) get 302 redirect. Dockerfile health check uses curl without Accept header, so it gets 401 JSON which causes health check to fail (container shows as unhealthy). Additionally, user expects root path to redirect when accessing from EC2 instance.
fix: Update Dockerfile health check to send Accept: text/html header so it gets 302 redirect instead of 401 JSON. This will make health check pass and match expected browser behavior.
verification: User confirmed curl behavior (401 JSON without Accept header; 302 redirect with Accept: text/html). Health check fix remains valid; image rebuild required to apply health check change.
files_changed: [packages/core/src/docker/Dockerfile]
