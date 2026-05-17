# Entry Points

File-by-file tour. Use this to translate a concern ("how does the API verify the token?") into a specific file you should open.

Pair with [`gotchas.md`](./gotchas.md) when debugging and [`extension-recipes.md`](./extension-recipes.md) when adding features.

## Web (`web/`, Next.js 16 App Router)

| File | What it owns | Read it when… |
|---|---|---|
| [`web/proxy.ts`](../../web/proxy.ts) | `clerkMiddleware()` + `createRouteMatcher(["/protected(.*)"])` — gates which routes require auth. Excludes `_next` and dotted assets. | You need to add a new protected URL prefix, or you're surprised auth is/isn't running. **Note: Next.js 16 renamed `middleware.ts` → `proxy.ts`.** |
| [`web/app/layout.tsx`](../../web/app/layout.tsx) | `<ClerkProvider>` wrapper, header sign-in/out controls. | Changing how Clerk is mounted, or where the sign-in button renders. |
| [`web/app/protected/page.tsx`](../../web/app/protected/page.tsx) | Server component: calls `auth().getToken()`, fetches `${NEXT_PUBLIC_API_BASE_URL}/me`, renders the JSON. | You want the canonical pattern for calling the Rust API from a Next.js Server Component or Route Handler. |
| [`web/.env.example`](../../web/.env.example) | The contract for what the web app needs at runtime. | You're spinning up a new environment — copy this to `web/.env.local` and fill in. |

## API (`api/`, Rust + Axum)

| File | What it owns | Read it when… |
|---|---|---|
| [`api/src/main.rs`](../../api/src/main.rs) | Router, CORS layer, `/health` (public), `/me` (protected), and where the auth middleware is applied. | Adding a new route, changing CORS, or moving a route between the public and protected route groups. |
| [`api/src/auth.rs`](../../api/src/auth.rs) | `JwksCache` (RwLock + TTL + `kid`-miss refresh), `verify_token`, the Axum middleware, the `Claims` extractor. | Anything to do with JWT verification, key rotation, or extending the claims shape. |
| [`api/src/error.rs`](../../api/src/error.rs) | `AppError -> IntoResponse` (401/403/500 with a JSON body). | Changing the wire shape of errors or adding a new error variant. |
| [`api/.env.example`](../../api/.env.example) | The contract for what the API needs at runtime — issuer URL, authorized parties, port. | Spinning up a new environment or debugging "why is verification failing?" |

## Top-level

| File | What it owns |
|---|---|
| [`README.md`](../../README.md) | First-time setup + smoke verification (`curl /health`, `curl /me` with and without a token). |
| [`docs/`](../) | The conceptual docs this map points into. See [`README.md`](./README.md) "Sibling docs" for the index. |

## Reading order for a new agent

1. Top-level [`README.md`](../../README.md) — get the system running locally.
2. [`README.md`](./README.md) in this folder — orient.
3. [`../authentication-flow.md`](../authentication-flow.md) — see one request go through end-to-end.
4. `web/proxy.ts` + `web/app/protected/page.tsx` — the entire web side fits on one screen each.
5. `api/src/main.rs` → `api/src/auth.rs` — follow the request from router to verifier.
6. Whatever deep-dive your task actually needs.

If a file you'd expect to find isn't in the tables above, check [`scope.md`](./scope.md) — it's probably intentionally absent.
