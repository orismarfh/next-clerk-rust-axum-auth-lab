# Architecture

## Goal

Demonstrate the canonical "Clerk-on-the-frontend, your-API-on-the-backend" pattern with a non-Node backend (Rust/Axum). The lab is intentionally minimal: one public endpoint, one protected endpoint, one protected page.

## End-to-end JWT flow

```
┌──────────┐  1. interactive sign-in           ┌──────────┐
│ Browser  │ ────────────────────────────────► │  Clerk   │
└────┬─────┘                                   └────┬─────┘
     │                                              │
     │  2. session cookie + Clerk JS in page        │
     │ ◄────────────────────────────────────────────┘
     │
     │  3. getToken() → short-lived RS256 JWT
     ▼
┌──────────┐  4. Authorization: Bearer <jwt>    ┌──────────────┐
│ Next.js  │ ─────────────────────────────────► │ Rust / Axum  │
│   web/   │                                    │     api/     │
└──────────┘                                    └──────┬───────┘
                                                       │
                                          5. verify    │
                                                       ▼
                                                 ┌────────────┐
                                                 │ Clerk JWKS │
                                                 └────────────┘
```

1. User signs in via Clerk-hosted components inside the Next.js app.
2. Clerk sets its session cookie and exposes a JS API for the page.
3. Before calling the API, the page calls `getToken()` (server-side `auth().getToken()` in App Router or client-side `useAuth().getToken()`). This returns a short-lived JWT signed by Clerk's per-app RSA key.
4. The browser/server includes the JWT in `Authorization: Bearer …` to the Rust API.
5. The Rust API:
   - Reads the `kid` from the JWT header.
   - Looks the key up in an in-memory JWKS cache; on miss or TTL expiry, fetches `${CLERK_JWT_ISSUER}/.well-known/jwks.json`.
   - Verifies signature (RS256), `iss == CLERK_JWT_ISSUER`, `exp/nbf` window, and that `azp` is in `CLERK_AUTHORIZED_PARTIES`.
   - Inserts the verified `Claims` into request extensions for downstream handlers.

## Key decisions

| Decision | Choice | Why |
|---|---|---|
| Token type | Clerk **session JWT** (`getToken()` with default template) | Stateless, no DB lookup needed, short TTL handled by Clerk. |
| Verification | **JWKS-based, local** | No per-request call to Clerk; scales; no shared secret. |
| Caching | In-memory `RwLock<Cache>` with TTL + on-`kid`-miss refresh | Simple, sufficient for a lab; documented extension point for Redis/dashmap later. |
| Cross-origin auth | `Authorization` header (no cookies on `api/`) | Avoids CSRF surface; matches how third-party apps would call the API. |
| CORS | `tower_http::cors::CorsLayer` allowing `CLERK_AUTHORIZED_PARTIES` origins + `Authorization` header | Mirrors `azp` allowlist to keep one source of truth. |

## Out of scope (so the next agent doesn't re-litigate)

- User mirror table / database persistence
- Clerk webhooks (`user.created` etc.)
- Refresh-token handling (Clerk SDK does it transparently)
- Role/permission checks beyond reading claims
- Cookie-based session forwarding between web and api

## Extension hooks

- **Add a role check:** wrap `/me` (or new routes) in an additional layer that inspects `claims.public_metadata.role` after the JWT layer.
- **Swap JWKS cache for Redis:** replace `auth::JwksCache` with anything that implements an equivalent `get(kid).await` API.
- **Mirror users into Postgres:** add a Clerk webhook receiver in `api/` and persist on `user.created`.
