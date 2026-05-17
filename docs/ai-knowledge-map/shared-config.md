# Shared Config

Variables that must agree across the `web/` and `api/` apps. Each row is a known-good pairing; mismatches surface as the failure mode listed in the last column.

Pair with [`gotchas.md`](./gotchas.md) for the symptom-first view.

## The contract

| Web side | API side | What must agree | Failure mode if it doesn't |
|---|---|---|---|
| `NEXT_PUBLIC_API_BASE_URL` (e.g. `http://localhost:8080`) | `API_PORT` (e.g. `8080`) | Host + port. The web app fetches `${NEXT_PUBLIC_API_BASE_URL}/me`; the API binds to `0.0.0.0:${API_PORT}`. | Browser console shows `fetch failed` / `ECONNREFUSED` on `/protected`. |
| Web app origin (`http://localhost:3000` by default) | `CLERK_AUTHORIZED_PARTIES` (comma-separated) | The web origin must appear in the API's allowlist **and** in the CORS allowed-origins (which is derived from the same env var). | Two distinct failures: (1) JWT verification rejects `azp` → **401** from `/me` with `azp_not_authorized`; (2) browser blocks the response → CORS error in console, but the API logs a 200. |
| Same Clerk app (publishable + secret keys) | `CLERK_JWT_ISSUER` (e.g. `https://xxxx.clerk.accounts.dev`) | Both sides must be talking about the same Clerk application. The issuer URL is per-app; mixing dev/prod or two different apps is the most common silent-misconfig. | Verification fails with `iss` mismatch or JWKS fetch returns a different key set → **401** (`invalid_issuer`) or **500** if the issuer host is unreachable / returns non-2xx. |

## How the values flow

```
Clerk Dashboard
   ├── Publishable key  ─► web/.env.local  ─► <ClerkProvider> in app/layout.tsx
   ├── Secret key       ─► web/.env.local  ─► server-side auth() in app/protected/page.tsx
   └── Issuer URL       ─► api/.env        ─► CLERK_JWT_ISSUER ─► JwksCache fetches ${issuer}/.well-known/jwks.json

Local dev only:
   web/ runs on :3000   ─► must appear in api/.env CLERK_AUTHORIZED_PARTIES
   api/ runs on :8080   ─► must match web/.env.local NEXT_PUBLIC_API_BASE_URL
```

## Adding a new origin (e.g. a staging URL)

1. Add it to **Clerk Dashboard → Domains → Allowed origins**. Without this, Clerk refuses to mint tokens for the origin.
2. Append it to `CLERK_AUTHORIZED_PARTIES` in `api/.env` (comma-separated, no spaces). This drives both the `azp` allowlist in `auth::verify_token` and the CORS allowed-origins in `main.rs`.
3. Restart the API. The env is read at boot; nothing watches the file.

If you skip step 1, sign-in itself fails on that origin. If you skip step 2, sign-in works but the first `/me` call returns 401.

## Things that look like they should be shared but aren't

- **`CLERK_SECRET_KEY` is web-only.** The API verifies via JWKS; it never needs the secret key. Adding it to `api/.env` is a footgun — see [`gotchas.md`](./gotchas.md).
- **`NEXT_PUBLIC_*` prefix is web-only.** Anything with that prefix is exposed to the browser bundle. Never apply it to a secret.
- **Cookies are not shared.** The API does not read Clerk's session cookie; the only auth signal it accepts is the `Authorization: Bearer …` header. This is intentional — see [`../architecture.md`](../architecture.md) "Key decisions".
