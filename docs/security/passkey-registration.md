# Passkey Registration Security (IOTP Bootstrap Flow)

During first-time setup, opencode-cloud requires the user to verify an **Initial One-Time Password (IOTP)** printed in the container logs and then enroll a **WebAuthn passkey** before any protected functionality is accessible. This document explains the security mechanisms that gate passkey registration and how they interact.

## Flow Overview

```
User reads IOTP from container logs
         │
         ▼
┌─────────────────────────────────────┐
│  POST /bootstrap/verify             │  IOTP validated by privileged helper
│  ─ HTTPS enforced                   │  Session created with:
│  ─ CSRF header required             │    bootstrapPending = true
│  ─ Rate limited (3 / 15 min)        │    bootstrapOtp = <verified OTP>
└──────────────┬──────────────────────┘
               │ redirect
               ▼
┌─────────────────────────────────────┐
│  GET /auth/passkey/setup            │  Passkey enrollment UI
│  ─ ?required=1 hides "Skip" button  │  (cosmetic only; real gate is
│  ─ If bootstrapPending && creds > 0 │   middleware, not query param)
│    → clears pending, redirects      │
└──────────────┬──────────────────────┘
               │ user initiates enrollment
               ▼
┌─────────────────────────────────────┐
│  POST /passkey/register/options     │  Server generates WebAuthn challenge
│  ─ Session required                 │  Returns:
│  ─ CSRF header required             │    - WebAuthn registration options
│  ─ HTTPS enforced                   │    - Signed JWT challenge token
└──────────────┬──────────────────────┘
               │ browser calls navigator.credentials.create()
               ▼
┌─────────────────────────────────────┐
│  POST /passkey/register/verify      │  Server verifies attestation
│  ─ Session required                 │  1. Consume challenge token (single-use)
│  ─ CSRF header required             │  2. Verify WebAuthn response
│  ─ HTTPS enforced                   │  3. Store credential
│                                     │  4. If bootstrapPending:
│                                     │     ─ Retrieve session.bootstrapOtp
│                                     │     ─ Call completeBootstrapOtp(otp)
│                                     │     ─ Clear bootstrapPending on success
└──────────────┬──────────────────────┘
               │
               ▼
        Bootstrap complete
        Protected routes accessible
```

## Security Layers

| Layer | What it prevents | Source |
|-------|-----------------|--------|
| HTTPS enforcement | Credential interception, MitM attacks | `fork-auth/src/security/https-detection.ts` |
| CSRF (`X-Requested-With` header) | Cross-site request forgery on all state-changing endpoints | `fork-auth/src/security/csrf.ts` |
| Rate limiting | Brute-force IOTP guessing (3 attempts per 15 minutes) | `fork-auth/src/security/rate-limit.ts` |
| Session-bound state | Ensures IOTP verification and passkey registration are tied to the same session | `opencode/src/session/user-session.ts` |
| Challenge token replay prevention | Reusing a WebAuthn challenge for multiple registrations | `fork-auth/src/auth/passkey-challenge.ts` |
| Middleware blocking | Accessing protected routes before passkey enrollment completes | `fork-auth/src/middleware/auth.ts` |
| Bootstrap completion validation | Marking bootstrap as done without a valid IOTP + enrolled passkey | `fork-auth/src/auth/bootstrap.ts` |

## Detailed Walkthrough

### Step 1: IOTP Verification (`POST /bootstrap/verify`)

**Source:** `fork-auth/src/routes/auth.ts` (the `/bootstrap/verify` handler)

Before anything else, the endpoint enforces:
- **HTTPS** (or localhost) via `shouldBlockInsecureLogin()`.
- **Rate limiting** via `bootstrapRateLimiter()` (3 failed attempts per 15 minutes).
- **CSRF** via `X-Requested-With` header check.

The OTP is validated by calling `verifyBootstrapOtp(otp)`, which invokes the privileged `opencode-cloud-bootstrap` helper binary via `sudo -n`. The helper runs outside the web process and is the sole authority on whether the IOTP is valid and the bootstrap flow is active.

On success:
1. A session is created for the `opencoder` user via `UserSession.create()`.
2. `UserSession.setBootstrapPending(session.id, otp)` sets two flags:
   - `bootstrapPending = true` — blocks all protected routes.
   - `bootstrapOtp = <the verified OTP>` — stored in session for later completion.
3. A session cookie and CSRF cookie are set.
4. The response includes `redirectTo: /auth/passkey/setup?required=1`.

On failure the helper returns specific error codes (`otp_invalid`, `inactive`, `helper_error`) and the rate limiter records the failure.

**Source:** `fork-auth/src/auth/bootstrap.ts` (`verifyBootstrapOtp`)
**Source:** `opencode/src/session/user-session.ts` (`setBootstrapPending`)

### Step 2: Passkey Setup Page (`GET /auth/passkey/setup`)

**Source:** `fork-auth/src/routes/auth.ts` (the `/passkey/setup` handler)

This endpoint serves the passkey enrollment UI. The `required` query parameter controls only whether a "Skip" button is shown in the UI:

```typescript
const required = session.bootstrapPending === true || c.req.query("required") === "1"
// ...
canSkip: !required,
```

**Removing `?required=1` from the URL has no security impact.** If the user clicks "Skip", they are redirected to the app — but the auth middleware immediately redirects them back to `/auth/passkey/setup?required=1` because `session.bootstrapPending` is still `true` (see [Middleware Enforcement](#middleware-enforcement)). The user is stuck in a loop until they register a passkey.

There is also an early-exit optimization: if `bootstrapPending` is true but the user already has registered credentials (e.g., from a retry), it clears the pending state and redirects to the app.

### Step 3: WebAuthn Challenge Generation (`POST /passkey/register/options`)

**Source:** `fork-auth/src/routes/auth.ts` (the `/passkey/register/options` handler)
**Source:** `fork-auth/src/auth/passkey-challenge.ts`

Requires an authenticated session, CSRF header, and HTTPS. Generates WebAuthn registration options via the `simplewebauthn` library and creates a signed challenge token.

The challenge token is a JWT (HS256) containing:
- `typ: "passkey_register"` — purpose binding.
- `challenge` — the WebAuthn challenge bytes.
- `rpID` / `origins` — relying party identity.
- `username` / `ip` — optional bindings.
- `jti` — unique token ID (UUID) for replay prevention.
- `exp` — expiration (default 5 minutes).

The token is signed with a server-held secret and returned to the client alongside the WebAuthn options.

### Step 4: Passkey Registration Verification (`POST /passkey/register/verify`)

**Source:** `fork-auth/src/routes/auth.ts` (the `/passkey/register/verify` handler)

This is the critical security gate. The flow is:

1. **Consume the challenge token** via `consumePasskeyChallengeToken()`:
   - Verifies JWT signature and expiration.
   - Checks the `jti` against an in-memory `used` map — if already consumed, returns `null`.
   - Optionally validates that the request IP matches the token's IP.
   - Marks the `jti` as used (preventing replay).

2. **Verify the WebAuthn attestation** via `verifyRegistrationResponse()`:
   - Validates the authenticator's attestation against the original challenge.
   - Extracts the public key and credential metadata.

3. **Store the credential** in persistent storage (survives server restarts).

4. **If `bootstrapPending` is true** (the IOTP bootstrap flow):
   - Retrieve `session.bootstrapOtp`. If missing, return 401 (`bootstrap_state_invalid`).
   - Call `completeBootstrapOtp(otp)` to finalize with the privileged helper.
   - If the helper reports failure (and the code is not `inactive`), return 500 — the passkey is stored but bootstrap is not marked complete.
   - On success, call `UserSession.clearBootstrapPending(session.id)` to clear both `bootstrapPending` and `bootstrapOtp`.

**Source:** `fork-auth/src/auth/bootstrap.ts` (`completeBootstrapOtp`)

## Challenge Token Security

**Source:** `fork-auth/src/auth/passkey-challenge.ts`

Challenge tokens prevent several attacks:

| Property | Mechanism |
|----------|-----------|
| **Signed** | HS256 JWT — cannot be forged without the server secret |
| **Time-limited** | `exp` claim, default 5 minutes |
| **Single-use** | `jti` tracked in an in-memory `Map`; second consumption returns `null` |
| **Purpose-bound** | `typ` field distinguishes `passkey_auth` from `passkey_register` |
| **IP-bound** (optional) | If `ip` is set in the token and the verifying request has a different IP, consumption fails |
| **Pruned** | Expired JTIs are pruned on each `consume` call to prevent memory growth |

## Middleware Enforcement

**Source:** `fork-auth/src/middleware/auth.ts`

The auth middleware runs on **all protected routes** and checks `session.bootstrapPending`:

```typescript
if (session.bootstrapPending) {
  if (isApiCall()) {
    return c.json({ error: "passkey_setup_required", message: "Passkey setup is required" }, 403)
  }
  return c.redirect("/auth/passkey/setup?required=1")
}
```

This means:
- No protected API endpoint can be called while bootstrap is pending (403).
- Browser requests are redirected to passkey setup.
- The only way to clear `bootstrapPending` is to complete the full passkey registration flow, which requires `completeBootstrapOtp()` to succeed.
- URL manipulation (removing `?required=1`, navigating directly to protected routes) does not bypass this check.

## Edge Cases

### Session expires during passkey setup

Sessions are in-memory and have a sliding expiration window. If the session expires mid-flow, the user must restart from IOTP verification. The IOTP helper tracks completion state independently, so a stale session cannot be used to skip verification.

### `completeBootstrapOtp()` fails after passkey is stored

The passkey credential is persisted before bootstrap completion is attempted. If the helper fails, the endpoint returns a 500 error, and `bootstrapPending` remains `true`. The user can retry passkey registration. On the next visit to `/auth/passkey/setup`, the early-exit check detects existing credentials and clears the pending state (since the passkey already exists).

### Missing `bootstrapOtp` in session

If `session.bootstrapOtp` is somehow unset while `bootstrapPending` is true, the endpoint returns 401 (`bootstrap_state_invalid`). The user must restart from the login page.

### Server restart during bootstrap

Sessions are in-memory and lost on restart. The user must re-verify the IOTP. The helper's state is independent — if the IOTP was already completed, the helper returns `inactive` and the user needs `occ reset iotp` to generate a fresh one.

### Concurrent bootstrap attempts

Rate limiting (3 attempts / 15 min) constrains parallel IOTP guessing from the same IP. Each successful verification creates an independent session, but `completeBootstrapOtp()` is idempotent at the helper level — only the first completion succeeds; subsequent calls return `inactive`.

## Key Source Files

| File | Role |
|------|------|
| `packages/opencode/packages/fork-auth/src/routes/auth.ts` | Auth route handlers (bootstrap verify, passkey setup, register options/verify) |
| `packages/opencode/packages/fork-auth/src/middleware/auth.ts` | `bootstrapPending` enforcement on all protected routes |
| `packages/opencode/packages/fork-auth/src/auth/passkey-challenge.ts` | JWT challenge token creation and single-use consumption |
| `packages/opencode/packages/fork-auth/src/auth/passkey.ts` | WebAuthn registration and authentication logic |
| `packages/opencode/packages/fork-auth/src/auth/passkey-storage.ts` | Credential persistence |
| `packages/opencode/packages/fork-auth/src/auth/bootstrap.ts` | IOTP helper communication (verify, complete, status) |
| `packages/opencode/packages/opencode/src/session/user-session.ts` | Session state management (`bootstrapPending`, `bootstrapOtp`) |
| `packages/opencode/packages/fork-auth/src/security/rate-limit.ts` | Rate limiting |
| `packages/opencode/packages/fork-auth/src/security/csrf.ts` | CSRF protection |
| `packages/opencode/packages/fork-auth/src/security/https-detection.ts` | HTTPS enforcement |
