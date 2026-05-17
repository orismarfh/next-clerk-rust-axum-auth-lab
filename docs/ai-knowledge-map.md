# AI Knowledge Map

> Purpose: orient a future agent quickly so they can extend this lab without re-reading every file. This file is the **only** doc you should need to read before opening source.

## TL;DR

- Two apps: `web/` (Next.js + Clerk) and `api/` (Rust + Axum).
- The frontend gets a JWT from Clerk (`getToken()`), sends it as `Authorization: Bearer`, and the Rust API verifies it locally against Clerk's JWKS.
- All secrets live in per-app `.env` files (gitignored). Templates: `web/.env.example`, `api/.env.example`.

## Entry points (read these first)

| Concern | File | What lives here |
|---|---|---|
| How auth works conceptually | [`docs/architecture.md`](./architecture.md) | Flow diagram + decisions + out-of-scope list |
| Backend bearer token handling (drill-down) | [`docs/backend-bearer-token.md`](./backend-bearer-token.md) | Wire contract, status-code matrix, code map, smoke recipes, extension hooks |
| Web sign-in routing | `web/proxy.ts` | `clerkMiddleware()` — Next.js 16 renamed `middleware.ts` → `proxy.ts`. `createRouteMatcher(["/protected(.*)"])` gates the route; matcher excludes `_next` and dotted assets. |
| Web protected page | `web/app/protected/page.tsx` | Server component: calls `auth().getToken()` then `fetch(API_BASE_URL + "/me")` |
| Web layout / provider | `web/app/layout.tsx` | `<ClerkProvider>` wrapper, header sign-in/out |
| API entrypoint | `api/src/main.rs` | Router, CORS, `/health`, `/me`, applies the auth layer |
| Token verification | `api/src/auth.rs` | JWKS cache + `verify_token` + Axum middleware/extractor + `Claims` struct |
| Error mapping | `api/src/error.rs` | `AppError -> IntoResponse` (401/403/500 with JSON body) |

## Variables that must match across apps

| Web side | API side | Why |
|---|---|---|
| `NEXT_PUBLIC_API_BASE_URL` | `API_PORT` | Base URL and port must agree (`http://localhost:${API_PORT}`). |
| Web app origin (`http://localhost:3000` by default) | `CLERK_AUTHORIZED_PARTIES` | Web origin must appear in the API's allowed `azp` set; mirror in CORS. |
| Same Clerk app | `CLERK_JWT_ISSUER` | API issuer URL must match the Clerk app the web uses. |

## Gotchas

- **`azp` mismatch → 401**: Clerk encodes the requesting origin in `azp`. If the web app runs on a port not in `CLERK_AUTHORIZED_PARTIES`, the API rejects the token. Symptom: valid sign-in, /protected page renders error from /me.
- **JWKS cache + key rotation**: cache TTL is 5 min by default; if Clerk rotates keys, first request after rotation triggers a refresh. If you see intermittent 401s after a long uptime, check the cache refresh path in `auth::JwksCache::get_or_refresh`.
- **`getToken()` returns `null`**: happens when the user isn't signed in or the session expired. Always check before calling the API; redirect to `/sign-in`.
- **CORS preflight**: browsers send `OPTIONS` for `Authorization`-bearing requests. `tower_http::cors::CorsLayer::very_permissive()` is too loose; the lab uses an explicit origin + headers + methods list.
- **Don't put `CLERK_SECRET_KEY` on the Rust side**: the API never needs it. Verification is JWKS-only.
- **Don't add `NEXT_PUBLIC_` to `CLERK_SECRET_KEY`**: that env prefix exposes vars to the browser bundle.
- **Next.js 16 renamed `middleware.ts` → `proxy.ts`**: don't grep for `middleware.ts` and conclude there's no auth middleware — the file is at `web/proxy.ts`. The dev-server log line `proxy.ts: 147ms` is the tell. Older Clerk docs and most training data still say `middleware.ts`.
- **Bad/unreachable Clerk issuer surfaces as HTTP 500, not 401**: in `api/src/auth.rs::JwksCache::refresh`, any non-2xx from the JWKS endpoint maps to `AppError::Internal`. So a misconfigured `CLERK_JWT_ISSUER` (e.g. typo, leftover placeholder) makes `/me` return `500 {"error":"internal_error","message":"JWKS endpoint returned 400 Bad Request"}` instead of the 401 you'd expect from an auth path. When debugging "auth not working", check `RUST_LOG=debug` for the `refreshing JWKS url=…` line first — the issuer in the URL is the source of truth.

## How to extend

| Goal | What to change |
|---|---|
| Add another protected route | New handler in `api/src/main.rs`, register under the route group that has the auth layer. Read `Claims` from request extensions. |
| Require a specific role | Add a second middleware in `api/src/auth.rs` that reads `Claims.public_metadata.role` (extend the struct) and returns 403 if missing. |
| Call the API from a Next.js Route Handler | Use `auth().getToken()` server-side and `fetch` with the bearer header. Same flow as `app/protected/page.tsx`. |
| Persist users | Add a Clerk webhook receiver in `api/`, hash-verify with the signing secret, upsert into your DB on `user.created`/`user.updated`. |
| Swap to another OIDC provider | The `auth.rs` module only assumes JWKS + RS256 + standard claims. Swap `CLERK_JWT_ISSUER` for the new issuer URL and adapt `Claims` shape. |

## What's intentionally missing

- No database, no migrations, no ORM
- No CI, no Dockerfile
- No tests beyond the smoke verification documented in `README.md`
- No Tailwind / no UI library — the protected page renders raw JSON to keep focus on auth

## Related issues

- [ZIZ-65](/ZIZ/issues/ZIZ-65) — this delivery
- [ZIZ-64](/ZIZ/issues/ZIZ-64) — parent (Inbox Board)
