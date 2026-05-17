# Extension Recipes

Concrete recipes for the most common ways future agents will extend this lab. Each recipe names the files to touch and the gotchas to read first.

For the broader "what's deliberately not here" list, see [`scope.md`](./scope.md).

## Recipe: add another protected route to the Rust API

**Files:** `api/src/main.rs` (route registration), `api/src/auth.rs` (only if you need new claim fields).

**Steps:**

1. Write the handler. Pull `Claims` out of request extensions — the auth middleware inserts them on success:
   ```rust
   async fn handler(claims: Claims) -> impl IntoResponse {
       Json(serde_json::json!({ "you_are": claims.sub }))
   }
   ```
2. Register the route **inside the route group that has the auth layer applied.** In `main.rs`, the protected group looks like `.route("/me", get(me_handler))` followed by `.layer(middleware::from_fn(require_auth))` — your new route must be in the same `Router` before the layer call, otherwise it's public.
3. Smoke: `curl -i http://localhost:8080/<your-route>` → expect 401. With a token → expect 200.

**Gotcha:** if you put the route after the layer call, or in a different `Router::new()` that you `.merge()` later, the layer won't apply. Always re-run the unauthenticated `curl` to confirm.

## Recipe: require a specific role

**Files:** `api/src/auth.rs` (extend `Claims`), `api/src/main.rs` (add a second middleware).

**Steps:**

1. In Clerk Dashboard → Sessions → Customize session token, add `public_metadata` (or a flatter `role` claim) to the JWT template.
2. Extend the `Claims` struct in `api/src/auth.rs` with the new field:
   ```rust
   #[derive(Deserialize, Clone)]
   pub struct Claims {
       pub sub: String,
       // …existing fields…
       pub public_metadata: Option<PublicMetadata>,
   }
   #[derive(Deserialize, Clone)]
   pub struct PublicMetadata { pub role: Option<String> }
   ```
3. Write a `require_role(role: &'static str)` middleware that reads `Claims` from extensions and returns `AppError::Forbidden` (403) if absent or mismatched. Layer it on top of `require_auth`, not instead of it.

**Gotcha:** roles in `public_metadata` are *public* — anyone with the token can decode them. Don't put secrets there. For sensitive metadata, use `private_metadata` and fetch from Clerk's API server-side instead of from the JWT.

## Recipe: call the API from a Next.js Route Handler

**Files:** new file under `web/app/api/<your-route>/route.ts`.

**Pattern:** same shape as `web/app/protected/page.tsx`, but in a Route Handler:

```ts
import { auth } from "@clerk/nextjs/server";

export async function GET() {
  const { getToken } = await auth();
  const token = await getToken();
  if (!token) return Response.json({ error: "unauthorized" }, { status: 401 });

  const res = await fetch(`${process.env.NEXT_PUBLIC_API_BASE_URL}/me`, {
    headers: { Authorization: `Bearer ${token}` },
    cache: "no-store",
  });
  return new Response(await res.text(), { status: res.status });
}
```

**Gotcha:** `getToken()` in a Route Handler is server-side and uses the session cookie — it doesn't need any client JS. For *client components*, use `useAuth().getToken()` instead and check for `null` before calling the API.

## Recipe: mirror users into your database on sign-up

**Files:** new module in `api/`, plus a route in `main.rs` (this one is **public** but signature-verified).

**Steps:**

1. In Clerk Dashboard → Webhooks, create an endpoint pointing at `${API_BASE}/webhooks/clerk` and subscribe to `user.created` / `user.updated` / `user.deleted`. Copy the signing secret.
2. Add `svix` (or hash-verify manually with `hmac-sha256`) and a handler that:
   - Reads the `svix-id`, `svix-timestamp`, `svix-signature` headers.
   - Verifies the signature against the raw body using the signing secret from env.
   - Parses the payload and upserts into your DB.
3. Register the route **outside** the auth-layered group — Clerk's webhooks don't carry a user JWT.

**Gotcha:** sign the *raw* request body, not a re-serialized JSON. Axum's `Json<T>` extractor consumes the body; use `Bytes` + manually parse after signature verification.

## Recipe: swap the in-memory JWKS cache for Redis (or dashmap)

**Files:** `api/src/auth.rs`.

**Pattern:** `JwksCache` exposes an `async fn get(&self, kid: &str) -> Option<DecodingKey>` (effectively). Anything implementing the same shape can be dropped in — keep the `kid`-miss refresh semantics so key rotation still works.

**Gotcha:** if you go cross-process (Redis), you need a per-process fallback for the case where Redis is down. The current in-memory cache is small and cheap enough that "Redis primary + in-memory fallback" is the right shape, not "Redis only".

## Recipe: swap to another OIDC provider (Auth0, Authentik, your own)

**Files:** `api/src/auth.rs` (`Claims` shape, `verify_token` validation), `api/.env.example`.

**What carries over:** JWKS fetch, RS256 verification, `iss` / `exp` / `nbf` checks — these are standard OIDC.

**What you'll change:**

- Issuer URL → from the new provider.
- The `azp` claim may not exist; replace the authorized-parties check with `aud` (audience) for most non-Clerk providers.
- The shape of `public_metadata` / custom claims will differ — keep the `Claims` struct provider-specific and don't pretend it's portable.

**Gotcha:** some providers serve JWKS at a non-`/.well-known/jwks.json` path. Check the provider's `/.well-known/openid-configuration` for the actual `jwks_uri`.
