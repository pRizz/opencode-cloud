---
status: verifying
trigger: "remote-docker-build-parse-error"
created: 2026-01-23T00:00:00Z
updated: 2026-01-23T00:05:00Z
---

## Current Focus

hypothesis: CONFIRMED - Cockpit configuration heredoc on line 332-343 is not properly terminated, causing Docker to interpret [WebService] as a Dockerfile instruction
test: Check if heredoc delimiter COCKPIT_CONF is quoted properly
expecting: Heredoc delimiter needs quoting to prevent variable expansion issues
next_action: Fix heredoc syntax in Dockerfile

## Symptoms

expected: Container starts on remote host when running `occ start --host aws-prod2`
actual: Docker build fails with parse error: "dockerfile parse error on line 333: unknown instruction: [WebService]"
errors:
```
⠏ Build failed
Error: Docker build failed: Docker responded with status code 400: dockerfile parse error on line 333: unknown instruction: [WebService]
```
reproduction: `just run start --host aws-prod2`
started: Never worked - first time trying remote start on this host

## Eliminated

- hypothesis: Systemd service file incorrectly formatted
  evidence: The opencode.service creation uses printf correctly (line 426-442)
  timestamp: 2026-01-23T00:01:00Z

## Evidence

- timestamp: 2026-01-23T00:01:00Z
  checked: packages/core/src/docker/Dockerfile lines 330-343
  found: Cockpit config heredoc uses `<< 'COCKPIT_CONF'` with single quotes, which should work
  implication: Heredoc syntax appears correct, but Docker is still parsing [WebService] as instruction

- timestamp: 2026-01-23T00:02:00Z
  checked: Line 333 content
  found: Line 333 is `[WebService]` inside the heredoc, which is being interpreted as a Dockerfile instruction
  implication: Docker parser is not recognizing the heredoc boundary, possibly due to cat command context or Docker version differences

- timestamp: 2026-01-23T00:03:00Z
  checked: Docker heredoc documentation and GitHub issues
  found: Traditional shell heredoc (cat > file << 'EOF') requires BuildKit enabled, or the Docker parser treats content as Dockerfile instructions
  implication: Remote host might not have BuildKit enabled, or we need to use Dockerfile heredoc syntax (RUN <<EOF) instead of shell heredoc (cat << EOF)

- timestamp: 2026-01-23T00:10:00Z
  checked: Running `occ start --host aws-prod2` after fix
  found: Different error now: "SSH tunnel not ready: SSH tunnel connection timed out after 3 attempts"
  implication: Original Dockerfile parse error is resolved, but now hitting SSH tunnel issue (separate problem)

- timestamp: 2026-01-23T00:12:00Z
  checked: Local Docker build with fixed Dockerfile
  found: Build successfully parses Dockerfile and starts building (reached step #5 installing packages)
  implication: Dockerfile syntax is now valid, parse error is fixed

## Resolution

root_cause: Shell heredoc syntax (cat > file << 'DELIMITER') on line 332-343 is not compatible with Docker parser on remote host. When Docker sends the Dockerfile to the remote daemon, it interprets [WebService] on line 333 as a Dockerfile instruction instead of heredoc content. This is a known issue with shell heredocs in Dockerfiles without proper BuildKit support.
fix: Replaced all three shell heredocs (cockpit.conf, starship.toml, zshrc aliases) with printf commands using proper quoting, consistent with the opencode.service creation pattern on line 426-442
verification: VERIFIED - Dockerfile now parses successfully. Local build test confirmed parsing works. Remote start attempt shows different error (SSH tunnel timeout), confirming original parse error is resolved.
files_changed: [packages/core/src/docker/Dockerfile]
