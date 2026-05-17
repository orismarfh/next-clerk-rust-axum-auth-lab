# Bearer Token Handling — Rust / Axum backend

Focused reference for how the `api/` service accepts and verifies a Clerk session JWT carried in `Authorization: Bearer …`. Pairs with `architecture.md` (end-to-end JWT flow) and `ai-knowledge-map.md` (where everything lives).

## Wire contract

The frontend must send:

```
GET /me HTTP/1.1
Host: localhost:8080
Authorization: Bearer eyJhbGciOiJSUzI1NiIsImtpZCI6Imluc18…
```

The backend returns:

| Condition | Status | Body |
|---|---|---|
| No `Authorization` header / no `Bearer ` prefix | `401` | `{ "error": "unauthorized", "message": "missing Bearer token" }` |
| Token unparseable, bad signature, expired, wrong issuer | `401` | `{ "error": "unauthorized", "message": "JWT verification failed: …" }` |
| Signature OK but `azp` not in allowlist (or absent) | `403` | `{ "error": "forbidden", "message": "azp '…' is not in the authorized parties allowlist" }` |
| JWKS endpoint unreachable / `kid` not found | `500` / `401` | `internal_error` if upstream JWKS errors; `unauthorized` for unknown `kid` |
| Verified, `/me` handler runs | `200` | Clerk claims (`sub`, `iss`, `exp`, `azp`, …) |

Public routes (`/health`) skip the layer entirely. Protected routes pick up the `require_auth` middleware via the protected `Router` group in `api/src/main.rs:49-51`.

## Code map

| Concern | Location |
|---|---|
| Extract `Bearer …` from header | `api/src/auth.rs:216-232` (`require_auth`) |
| RS256 + JWKS verification | `api/src/auth.rs:171-208` (`verify_token`) |
| JWKS cache (5 min TTL, refresh on `kid` miss) | `api/src/auth.rs:81-169` (`JwksCache`) |
| Strongly-typed claims | `api/src/auth.rs:50-65` (`Claims`) |
| Claims extractor for handlers | `api/src/auth.rs:234-248` (`FromRequestParts`) |
| HTTP error → JSON mapping | `api/src/error.rs` |
| Wiring (CORS, public vs protected) | `api/src/main.rs:38-57` |

## Step-by-step (one request)

1. **Header extraction.** `require_auth` reads `Authorization`, accepts either `Bearer ` or `bearer ` (case-tolerant prefix, single space), then trims surrounding whitespace. Missing / malformed → `401 unauthorized: missing Bearer token`. No other header (cookie, custom `X-…`) is consulted — this is intentional, see *Why no cookies* below.
2. **Header decode.** `decode_header` reads only the unverified JWT header to pluck `kid`. Missing `kid` → `401`.
3. **Key lookup.** `JwksCache::get_or_refresh(kid)` returns a cached `DecodingKey` if the cache is younger than `JWKS_TTL` (5 min) and the `kid` is present. Otherwise it refetches `${CLERK_JWT_ISSUER}/.well-known/jwks.json`, repopulates the cache, and retries the lookup. Unknown `kid` after refresh → `401 unknown key id: …`.
4. **Signature + standard claims.** `jsonwebtoken::decode` enforces `RS256`, the `iss` allowlist (`CLERK_JWT_ISSUER`), and the `exp` window (plus `nbf` if present). Any failure → `401 JWT verification failed: …`.
5. **Origin enforcement.** Clerk encodes the requesting web origin in `azp`. We require `azp` to be in `CLERK_AUTHORIZED_PARTIES`. Absent or mismatched → `403 forbidden`. This is the second source of truth for "who can call this API" (CORS is the other; both are configured from the same env var).
6. **Hand-off to the handler.** Verified `Claims` are inserted into the request extensions. Handlers either accept the `Claims` extractor (`async fn me(claims: Claims)`) or read them out of extensions manually.

## Why no cookies, why no introspection call

- **No cookies on `api/`.** Cookie auth across origins requires CSRF protections and same-site juggling. The Bearer header model matches how third-party apps (mobile, server-to-server, scripts) would call this API and keeps the API stateless.
- **No `/v1/sessions/{id}/verify` call to Clerk per request.** JWKS-only verification is local-only after the first key fetch. This scales to arbitrary RPS without coupling latency or rate limits to Clerk.

## Configuration

| Env var | Purpose | Example |
|---|---|---|
| `CLERK_JWT_ISSUER` | Used as the JWT `iss` allowlist and as the JWKS host (`${issuer}/.well-known/jwks.json`). Trailing `/` is stripped. | `https://xxxx.clerk.accounts.dev` |
| `CLERK_AUTHORIZED_PARTIES` | Comma-separated `azp` allowlist; also used to seed CORS `allow_origin`. Must list the web origin(s). | `http://localhost:3000` |
| `API_PORT` | Port the Axum server binds. | `8080` |

`CLERK_SECRET_KEY` is **not** required by the backend — verification is purely public-key. Putting it here would only widen blast radius.

## Local smoke (no browser)

```bash
# 1. Public route — no auth needed.
curl -s http://localhost:8080/health
# {"status":"ok"}

# 2. Protected route, no header.
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/me
# 401

# 3. Protected route, garbage token.
curl -s -H "Authorization: Bearer not-a-jwt" http://localhost:8080/me
# {"error":"unauthorized","message":"JWT verification failed: …"}

# 4. Real token — paste from browser console after sign-in:
#    await window.Clerk.session.getToken()
TOKEN="paste-here"
curl -s -H "Authorization: Bearer $TOKEN" http://localhost:8080/me | jq
# {"sub":"user_…","iss":"https://…","exp":…,"azp":"http://localhost:3000",…}
```

If step 4 returns `403 forbidden: azp '…' is not in the authorized parties allowlist`, your web origin isn't in `CLERK_AUTHORIZED_PARTIES`. Fix the env var, restart the API.

## Extension hooks (without touching the middleware)

- **Role / permission gate:** add a second `axum::middleware::from_fn` after `require_auth` that reads `Claims` from request extensions and 403s if a claim is missing.
- **Per-route opt-out:** keep adding routes to the public `Router` group; only the protected group runs `require_auth`.
- **Different verification source (custom OIDC):** swap `CLERK_JWT_ISSUER` for the new issuer and re-skin `Claims`. The middleware and cache are issuer-agnostic.
- **Cache backend swap:** replace `JwksCache` with anything exposing `get_or_refresh(&str) -> Result<DecodingKey, AppError>`. The middleware does not assume in-memory storage.

## Tests

`api/src/auth.rs` includes unit tests for the header-extraction contract (`Bearer`/`bearer` prefix, whitespace handling, missing header). Run them with:

```bash
cd api
cargo test
```

Full RS256 + JWKS verification is exercised end-to-end against a real Clerk token via the smoke script above; we deliberately do not stub Clerk in unit tests to keep the lab one moving part.
