# Authentication Flow

End-to-end walkthrough of what actually happens when a user goes from anonymous to calling the Rust API with a verified Clerk JWT. Read this when you want to understand *what runs where, in what order, with which data*. For the higher-level architectural decisions see [`architecture.md`](./architecture.md); for the backend-only reference see [`backend-bearer-token.md`](./backend-bearer-token.md); for a file tour see [`ai-knowledge-map.md`](./ai-knowledge-map.md).

## Actors

| Actor | Lives | Role |
|---|---|---|
| Browser | User's machine | Hosts the Next.js page, holds the Clerk session cookie, eventually carries the JWT on outbound fetches. |
| Next.js (`web/`) | `localhost:3000` | App Router app. Renders the UI, runs `clerkMiddleware` for route protection, and acts as the *server-side caller* of the Rust API (the `/protected` page is a server component). |
| Clerk | `*.clerk.accounts.dev` | Identity provider. Hosts sign-in/up UI, mints session JWTs (RS256), publishes the JWKS the API uses to verify them. |
| Rust API (`api/`) | `localhost:8080` | Axum service. Verifies the JWT locally against the Clerk JWKS and returns the claims from `/me`. |

## Sequence (happy path)

```
Browser              Next.js (web/)             Clerk                  Rust API (api/)
  │                       │                       │                          │
  │ 1. GET /protected     │                       │                          │
  ├──────────────────────►│                       │                          │
  │                       │ 2. clerkMiddleware                                │
  │                       │    proxy.ts: auth.protect()                       │
  │                       │    no session?                                    │
  │ 3. 307 → /sign-in     │                       │                          │
  │◄──────────────────────┤                       │                          │
  │                                                                          │
  │ 4. sign in (Clerk-hosted UI / modal)          │                          │
  ├──────────────────────────────────────────────►│                          │
  │ 5. session cookie set (__session) + JS bridge │                          │
  │◄──────────────────────────────────────────────┤                          │
  │                                                                          │
  │ 6. GET /protected (now with __session cookie) │                          │
  ├──────────────────────►│                                                  │
  │                       │ 7. clerkMiddleware: auth.protect() passes        │
  │                       │ 8. ProtectedPage (server component):             │
  │                       │      const { getToken } = await auth();          │
  │                       │      const token = await getToken();             │
  │                       │ 9. Clerk SDK mints a short-lived RS256 JWT       │
  │                       │     from the session cookie                      │
  │                       │                                                  │
  │                       │ 10. fetch(`${API_BASE}/me`,                      │
  │                       │       { headers: { Authorization: `Bearer …` }}) │
  │                       │──────────────────────────────────────────────────►│
  │                       │                                                  │ 11. require_auth middleware
  │                       │                                                  │     extract_bearer_token(headers)
  │                       │                                                  │     decode_header → kid
  │                       │                                                  │     JwksCache::get_or_refresh(kid)
  │                       │                                                  │       │
  │                       │                                                  │       │ 12. (cache miss / TTL) fetch
  │                       │                                                  │       │     ${ISSUER}/.well-known/jwks.json
  │                       │                                                  │       ├────────────────────────────►
  │                       │                                                  │       │◄────────────────────────────
  │                       │                                                  │     verify RS256 + iss + exp
  │                       │                                                  │     check azp ∈ allowlist
  │                       │                                                  │     inject Claims into extensions
  │                       │                                                  │ 13. me(claims: Claims) → 200 JSON
  │                       │◄──────────────────────────────────────────────────│
  │                       │ 14. ProtectedPage renders status + body          │
  │ 15. HTML response     │                                                  │
  │◄──────────────────────┤                                                  │
```

Subsequent calls re-use the Clerk session cookie (browser → Next.js) and Clerk's short-lived JWT (Next.js → Rust). The JWKS cache amortises the JWKS fetch across all subsequent token verifications for the next 5 minutes.

## Step-by-step

### 1. Anonymous request to a protected page

Browser navigates to `/protected`. Next.js runs `web/proxy.ts` first — this is the file the rest of the world still calls `middleware.ts`. (Next.js 16 renamed it. Old Clerk docs and most LLM training data say `middleware.ts`; in this repo it lives at `web/proxy.ts`.)

```ts
// web/proxy.ts
const isProtected = createRouteMatcher(["/protected(.*)"]);
export default clerkMiddleware(async (auth, req) => {
  if (isProtected(req)) await auth.protect();
});
```

`auth.protect()` short-circuits with a redirect to `/sign-in` when there is no Clerk session cookie. The `matcher` excludes `_next` assets and any path containing a dot, so static assets are never gated.

### 2. Sign-in via Clerk

`ClerkProvider` in `web/app/layout.tsx:22` wraps the app and exposes the modal sign-in / sign-up buttons in the header. Clerk hosts the actual sign-in form (modal or `/sign-in` page). On success Clerk:

- Sets the `__session` cookie on the Next.js origin.
- Establishes its in-page JS so `auth()` server-side and `useAuth()` client-side both work.

No JWT is minted at sign-in. The session cookie is the long-lived bearer-of-identity; JWTs are minted on demand.

### 3. JWT minting (server side)

`web/app/protected/page.tsx` is a server component. It calls:

```ts
const { getToken } = await auth();
const token = await getToken();
```

`auth()` reads the session cookie. `getToken()` asks Clerk for a short-lived RS256 JWT signed with the app's Clerk private key. Default template, no extra claims — the lab does not depend on custom session claims.

If `getToken()` returns `null` the user is not signed in (cookie expired, signed out in another tab, etc.). The page renders "Not signed in." — production code should redirect to `/sign-in`.

### 4. Outbound fetch with the bearer header

```ts
const res = await fetch(`${base}/me`, {
  headers: { Authorization: `Bearer ${token}` },
  cache: "no-store",
});
```

`cache: "no-store"` is deliberate: the JWT is short-lived and the per-user response must not be cached. The request goes **directly from the Next.js server to the Rust API**; the browser never sees the JWT in this flow. (When called from the client — e.g. `useAuth().getToken()` inside a Client Component — the JWT goes browser → Rust API instead; CORS still applies, see step 6.)

### 5. CORS preflight (browser-initiated calls only)

If you later fetch from a Client Component, the browser will preflight: `OPTIONS /me` with `Access-Control-Request-Headers: authorization`. `api/src/main.rs:43-46` allows the configured origins and the `Authorization` + `Content-Type` headers explicitly. Server-to-server calls (step 4 above) skip preflight entirely.

### 6. Bearer extraction

`api/src/auth.rs:216-224` — `extract_bearer_token`:

- Accepts `Bearer ` or `bearer ` (single space, case-tolerant prefix).
- Trims surrounding whitespace.
- Empty / missing / wrong-scheme → `401 unauthorized: missing Bearer token`.

This is covered by the unit tests in `api/src/auth.rs:253-302`.

### 7. JWT header decode + key lookup

`api/src/auth.rs:171-182` — `verify_token`:

```rust
let header = decode_header(token)?;            // unverified read of `kid`
let kid = header.kid.ok_or(...)?;
let key = cache.get_or_refresh(&kid).await?;
```

`get_or_refresh` (`auth.rs:104-127`) returns a cached `DecodingKey` if the cache is younger than `JWKS_TTL` (5 minutes) **and** the `kid` is present. Otherwise it calls `refresh()` which fetches `${CLERK_JWT_ISSUER}/.well-known/jwks.json`, repopulates the cache, and retries the lookup. Unknown `kid` after refresh → `401 unknown key id: …`. JWKS endpoint non-2xx → `500 internal_error` (the lab does not retry — see *Failure modes* below).

### 8. Signature + standard claims

```rust
let mut validation = Validation::new(Algorithm::RS256);
validation.set_issuer(&[cfg.issuer.as_str()]);
validation.set_required_spec_claims(&["exp", "iss"]);
let data = decode::<Claims>(token, &key, &validation)?;
```

`jsonwebtoken` enforces:

- `alg = RS256`
- `iss == CLERK_JWT_ISSUER`
- `exp` is in the future (default leeway 60s)
- `nbf` in the past if present

Anything else → `401 JWT verification failed: …`.

### 9. Origin enforcement (`azp`)

```rust
match &claims.azp {
    Some(azp) if cfg.authorized_parties.iter().any(|p| p == azp) => {}
    Some(azp) => return Err(AppError::Forbidden(...)),  // 403
    None      => return Err(AppError::Forbidden(...)),  // 403
}
```

Clerk encodes the requesting web origin in `azp`. The API rejects any token whose `azp` is not in `CLERK_AUTHORIZED_PARTIES`. This is the **second** source of truth for "who can call this API" — CORS is the first. Both are configured from the same env var so they cannot drift.

### 10. Claims hand-off

`require_auth` (`auth.rs:226-235`) inserts the verified `Claims` into `req.extensions_mut()` and calls the next layer. Handlers either use the `Claims` extractor (`async fn me(claims: Claims)`) — implemented via `FromRequestParts` at `auth.rs:237-251` — or read the extensions manually.

### 11. Handler runs

`api/src/main.rs:74-76`:

```rust
async fn me(claims: Claims) -> Result<Json<Claims>, AppError> {
    Ok(Json(claims))
}
```

The lab echoes the claims so you can see exactly what the API sees. Real code would do whatever it needs with `claims.sub`, etc.

## Token shape (what the API sees)

After verification, `Claims` (`api/src/auth.rs:50-65`) contains:

| Field | Type | Notes |
|---|---|---|
| `sub` | string | Clerk user id (`user_…`). The canonical "who is this". |
| `iss` | string | Clerk issuer URL. Already enforced by validation. |
| `exp` | usize | Unix seconds. Already enforced by validation. |
| `nbf` | optional usize | Unix seconds. Enforced if present. |
| `iat` | optional usize | Issued-at. |
| `azp` | optional string | Web origin. Required and enforced against the allowlist. |
| `sid` | optional string | Clerk session id (useful for revocation lookups; not used by the lab). |
| `extra` | flattened map | Anything else Clerk includes (e.g. custom session-template claims). Use `claims.extra.get("...")` to read. |

## Failure modes (status-code matrix)

| Trigger | Where it fails | Status | Body |
|---|---|---|---|
| User not signed in, hits `/protected` | `web/proxy.ts` (`auth.protect()`) | 307 | redirect to `/sign-in` |
| `getToken()` returns `null` (session expired) | `web/app/protected/page.tsx` | 200 | "Not signed in." (lab) — production should redirect |
| Missing / wrong-scheme `Authorization` header | `extract_bearer_token` | 401 | `{ "error": "unauthorized", "message": "missing Bearer token" }` |
| Malformed JWT / bad signature / expired / wrong `iss` | `verify_token` | 401 | `{ "error": "unauthorized", "message": "JWT verification failed: …" }` |
| JWT header has no `kid` | `verify_token` | 401 | `{ "error": "unauthorized", "message": "JWT header missing kid" }` |
| `kid` not present after JWKS refresh | `JwksCache::get_or_refresh` | 401 | `{ "error": "unauthorized", "message": "unknown key id: …" }` |
| Valid JWT but `azp` not in allowlist (or absent) | `verify_token` | 403 | `{ "error": "forbidden", "message": "azp '…' is not in the authorized parties allowlist" }` |
| JWKS endpoint unreachable / non-2xx (typo in issuer URL is the common cause) | `JwksCache::refresh` | 500 | `{ "error": "internal_error", "message": "JWKS endpoint returned …" }` |
| CORS preflight for an origin not in `CLERK_AUTHORIZED_PARTIES` | `CorsLayer` | 403 (browser blocks) | (no response body — surfaces as network/CORS error in devtools) |

A misconfigured `CLERK_JWT_ISSUER` typically surfaces as a **500**, not a 401 — see the gotcha in `ai-knowledge-map.md`. Enable `RUST_LOG=debug` and check for `refreshing JWKS url=…` to see the exact URL the API is trying.

## Session lifetime

| Surface | TTL | Refresh behaviour |
|---|---|---|
| Clerk session cookie (`__session`) | configurable in Clerk dashboard (default ~7 days, sliding) | refreshed by Clerk transparently on activity |
| Clerk JWT from `getToken()` | ~60 s by default | Clerk SDK mints a fresh one per call; treat tokens as single-use |
| JWKS cache (Rust) | 5 min (`JWKS_TTL` in `auth.rs:81`) | refreshed on TTL expiry or on `kid` miss |

Implication: you can't "store" a Clerk JWT and reuse it later. Always call `getToken()` immediately before the fetch.

## Manual end-to-end smoke

```bash
# 1. Public route, no auth.
curl -s http://localhost:8080/health
# → {"status":"ok"}

# 2. Protected route, no token.
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/me
# → 401

# 3. Protected route, real Clerk token.
#    In the browser, after signing in:
#      await window.Clerk.session.getToken()
TOKEN="paste-jwt-here"
curl -s -H "Authorization: Bearer $TOKEN" http://localhost:8080/me | jq
# → {"sub":"user_…","iss":"https://…","exp":…,"azp":"http://localhost:3000",…}
```

If step 3 returns `403 forbidden: azp '…' is not in the authorized parties allowlist`, the web origin you signed in on is not in `CLERK_AUTHORIZED_PARTIES`. Fix the env var and restart the API.

## Why this shape and not another

| Question | Choice | Reason |
|---|---|---|
| Why JWKS, not Clerk's session-introspection API? | JWKS-only verification | Local after first key fetch; scales to arbitrary RPS without coupling latency or rate limits to Clerk. |
| Why bearer header, not cookies on `api/`? | `Authorization: Bearer …` | Avoids CSRF surface; matches how a third-party/mobile/server-to-server client would call the API; keeps API stateless. |
| Why server-side `getToken()` in the page? | `auth().getToken()` in a server component | JWT never enters the browser, so it cannot leak via `window`, XSS, devtools, etc. Client-side `useAuth().getToken()` is fine when the caller is a Client Component, with the same verification path on the API. |
| Why `azp` allowlist *and* CORS? | Both, seeded from one env var | CORS is enforced by the browser only; `azp` is enforced server-side and protects non-browser callers too. |
| Why not `CLERK_SECRET_KEY` on the Rust side? | Not configured | The API does not call Clerk admin APIs; verification is purely public-key. Putting the secret there would only widen blast radius. |

## What is *not* covered by this flow

- User mirror table / DB persistence (no DB in this lab).
- Clerk webhooks (e.g. `user.created`).
- Refresh-token handling (Clerk SDK does it transparently in step 3).
- Role / permission checks beyond reading raw claims — see *Extension hooks* in `architecture.md` and `backend-bearer-token.md` for how to layer those in.
- Cookie-based session forwarding between web and api.

## Related issues

- [ZIZ-71](/ZIZ/issues/ZIZ-71) — this delivery (full auth-flow doc)
- [ZIZ-70](/ZIZ/issues/ZIZ-70) — backend bearer-token reference (`backend-bearer-token.md`)
