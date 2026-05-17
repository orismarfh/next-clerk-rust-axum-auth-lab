# Gotchas

Known traps. Organized symptom-first so you can grep for the error you're seeing.

For the wider failure-mode matrix see [`../authentication-flow.md`](../authentication-flow.md) "Failure modes" and [`../architecture-diagrams.md`](../architecture-diagrams.md) "Failure-mode map".

## Auth-flow gotchas

### Symptom: signed in, but `/me` returns 401 with `azp_not_authorized`

**Cause:** the web app's origin is not in `CLERK_AUTHORIZED_PARTIES`. Clerk encodes the requesting origin into the `azp` claim, and the API rejects tokens whose `azp` isn't in the allowlist.

**Fix:** add the origin to `api/.env` `CLERK_AUTHORIZED_PARTIES` (and Clerk Dashboard → Domains → Allowed origins if not already there). Restart the API. See [`shared-config.md`](./shared-config.md) "Adding a new origin".

### Symptom: `/me` returns **500** `{"error":"internal_error","message":"JWKS endpoint returned 400 Bad Request"}` instead of 401

**Cause:** `CLERK_JWT_ISSUER` is wrong (typo, leftover placeholder, dev/prod swap). In `api/src/auth.rs::JwksCache::refresh`, any non-2xx from the JWKS endpoint maps to `AppError::Internal`, **not** 401. So a misconfigured issuer looks like a server bug rather than an auth bug.

**Fix:** run the API with `RUST_LOG=debug` and grep for `refreshing JWKS url=…`. The issuer in that URL is the source of truth — compare it against the value in Clerk Dashboard → API Keys → Advanced → JWT public key → Issuer URL.

### Symptom: intermittent 401s after long uptime

**Cause:** JWKS cache TTL (default 5 min) expired around the same time Clerk rotated keys. The first request after rotation triggers a refresh; if that refresh races with verification, you can see a single 401 before the new keys land.

**Fix:** usually self-heals on retry. If it persists, look at `auth::JwksCache::get_or_refresh` and consider shortening the TTL or pre-warming on boot.

### Symptom: `getToken()` returns `null` on the web side

**Cause:** the user isn't signed in, or the session expired. This is a normal state, not a bug.

**Fix:** always check the return value before calling the API. The pattern in `web/app/protected/page.tsx` is the reference; for client components, redirect to `/sign-in` on `null`.

## CORS gotchas

### Symptom: API logs a 200 but the browser shows a CORS error

**Cause:** the response is fine, but its CORS headers don't allow the requesting origin. Browsers send a preflight `OPTIONS` for any request with a non-simple header like `Authorization`, and the preflight must explicitly allow the method, headers, and origin.

**Fix:** the lab uses an explicit origin + headers + methods list (`tower_http::cors::CorsLayer` in `api/src/main.rs`), driven by `CLERK_AUTHORIZED_PARTIES`. Don't replace it with `CorsLayer::very_permissive()` — that allows `*` but blocks credentials and is incompatible with `Authorization` in some browsers. Instead, add your origin to `CLERK_AUTHORIZED_PARTIES`.

## Framework / tooling gotchas

### Symptom: "where is the auth middleware?" — `grep middleware.ts` returns nothing

**Cause:** **Next.js 16 renamed `middleware.ts` → `proxy.ts`.** Older Clerk docs and most training data still say `middleware.ts`, so it's easy to conclude there's no auth middleware.

**Fix:** the file is at `web/proxy.ts`. The dev-server log line `proxy.ts: 147ms` is the tell. If you're following a tutorial that says "create `middleware.ts`", create `proxy.ts` instead.

## Secret-handling gotchas

### Don't put `CLERK_SECRET_KEY` on the Rust side

The API verifies via JWKS only — it never needs the secret key. Adding it to `api/.env` increases the blast radius for no benefit.

### Don't prefix `CLERK_SECRET_KEY` with `NEXT_PUBLIC_`

That prefix exposes the variable to the browser bundle, which leaks the secret to every visitor.

### `.env` files are gitignored — don't `git add -A` them

`.env.local` and `.env` are intentionally outside the repo. If you accidentally commit one, rotate the keys in Clerk before pushing — `git rm --cached` alone doesn't clean history.
