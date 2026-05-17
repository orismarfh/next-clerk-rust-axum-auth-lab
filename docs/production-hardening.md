# Production Hardening Checklist

> This lab is intentionally a **reference**, not a production deployment. The code in `api/` and `web/` is the minimum that proves the Clerk → Rust/Axum bearer flow works. This document is the bridge between "it runs on my laptop" and "it serves real traffic" — each item names what the lab does today, what to add, and why it matters.

Use it as a pre-launch gate: every line item should be ticked, deferred-with-justification, or explicitly out of scope before going live.

## How to read each row

- **State** — `lab` (present in this repo), `partial` (exists but insufficient), `missing` (must be added).
- **Owner** — the layer that has to fix it: `api`, `web`, `infra`, `clerk`, `process`.
- **Why** — the failure mode you are buying down. If you cannot articulate the failure mode, the item is not actually hardening.

---

## 1. Identity & token handling

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 1.1 | JWT signature verified locally against Clerk JWKS (no per-request callout) | lab | api | Latency, availability, and rate-limit isolation from Clerk. See `api/src/auth.rs:171`. |
| 1.2 | `iss` strictly pinned to `CLERK_JWT_ISSUER` | lab | api | Prevents tokens from another Clerk tenant being accepted. `auth.rs:185`. |
| 1.3 | `exp` required and enforced | lab | api | `Validation::set_required_spec_claims(&["exp","iss"])` at `auth.rs:186`. |
| 1.4 | `nbf` / `iat` enforced when present | partial | api | `jsonwebtoken` validates `nbf` by default with 60s leeway. Make it required for prod: `validation.set_required_spec_claims(&["exp","iss","nbf","iat"])`. |
| 1.5 | `azp` allowlist mirrors CORS allowlist | lab | api | One source of truth for "who is allowed to call us". `auth.rs:193`. |
| 1.6 | Reject tokens older than `MAX_TOKEN_AGE` (e.g. 5 min) as defence-in-depth | missing | api | Limits blast radius if a token leaks; complements Clerk's own short TTL. Compute from `iat`. |
| 1.7 | JWKS cache TTL with on-`kid`-miss refresh | lab | api | `JWKS_TTL = 5 min`; miss triggers refresh. `auth.rs:81`. |
| 1.8 | JWKS refresh resilient to transient failure (retry, backoff, jitter) | missing | api | Today a single 5xx surface from Clerk JWKS → all requests fail until next request retries. Add bounded retry + last-good-cache fallback for the TTL window. |
| 1.9 | JWKS refresh is single-flighted across concurrent misses | missing | api | Thundering-herd: N concurrent unknown-`kid` requests = N JWKS fetches. Use `tokio::sync::OnceCell` or a `Mutex` around `refresh()`. |
| 1.10 | Algorithm pinned to `RS256` (no `alg: none`, no HS family) | lab | api | `Validation::new(Algorithm::RS256)` at `auth.rs:184`. Never accept the alg from the header. |
| 1.11 | Token never logged | lab | api | Confirm log fields. `TraceLayer` does not log headers by default; keep it that way. |
| 1.12 | Clerk session token template reviewed (no PII, no secrets in custom claims) | process | clerk | Custom claims end up in every request and every log. Audit the template in the Clerk dashboard before shipping. |
| 1.13 | Token revocation / session-end behaviour understood | process | clerk | Clerk JWTs are short-lived (~60s). For a higher-trust system, gate critical actions on a server-side session check via Clerk's API. |

## 2. API surface (Rust/Axum)

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 2.1 | Request body size limit | missing | api | Axum's default is generous. Add `tower_http::limit::RequestBodyLimitLayer::new(64 * 1024)` (tune per route). |
| 2.2 | Per-request timeout | missing | api | Add `tower::timeout::TimeoutLayer::new(Duration::from_secs(10))`. Prevents slow-loris-style resource hold. |
| 2.3 | Concurrency cap | missing | api | `tower::limit::ConcurrencyLimitLayer::new(N)` per instance. Pair with autoscaling on saturation. |
| 2.4 | Rate limiting per-IP / per-`sub` | missing | api | `tower_governor` crate or edge WAF. Critical for `/me`-style endpoints that hit JWKS cache. |
| 2.5 | Security headers (`x-content-type-options`, `referrer-policy`, `strict-transport-security`, `x-frame-options`) | missing | api | `tower_http::set_header::SetResponseHeaderLayer`. HSTS is required when terminating TLS at the API. |
| 2.6 | CORS allowlist is explicit (no `*`) | lab | api | `cors_origins` is built from `CLERK_AUTHORIZED_PARTIES`. `main.rs:38`. |
| 2.7 | Errors do not leak internal detail | partial | api | `AppError::Internal(String)` returns the raw message in the JSON body (`error.rs:24`). Replace the user-facing body with a generic string; keep the detail in `tracing::error!` only. |
| 2.8 | 401 vs 403 are correct | lab | api | `Unauthorized` = bad/missing token, `Forbidden` = good token, wrong `azp`. Keep this distinction; it matters for client retry logic. |
| 2.9 | Graceful shutdown drains in-flight requests | missing | api | Use `axum::serve(...).with_graceful_shutdown(signal)` listening on SIGTERM. Required for zero-downtime deploys behind a load balancer. |
| 2.10 | Panics don't tear down the process | missing | api | Set `std::panic::set_hook` to log + emit a metric; rely on the runtime to restart only the task, not the whole instance. |
| 2.11 | `/health` is liveness only; add `/ready` that checks JWKS reachability | missing | api | Liveness and readiness must be different. A pod that cannot reach Clerk JWKS should fail readiness, not liveness (which would just restart it into the same broken state). |
| 2.12 | TLS terminated in front of the API (LB / ingress) | infra | infra | The app binds plain HTTP on `0.0.0.0:8080`. Put a TLS-terminating proxy in front; never expose plain HTTP to the internet. |
| 2.13 | HTTP/2 enabled at the edge | infra | infra | Lower overhead for the typical "one Bearer call per page" pattern. |

## 3. Web app (Next.js)

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 3.1 | Clerk middleware protects all non-public routes | lab | web | `proxy.ts:5` calls `auth.protect()` for `/protected(.*)`. Audit `createRouteMatcher` whenever you add a new route. |
| 3.2 | Token fetched server-side and forwarded as Bearer | lab | web | `app/protected/page.tsx:5` uses server-side `auth().getToken()`. Avoid exposing tokens to client JS unless required. |
| 3.3 | `NEXT_PUBLIC_API_BASE_URL` is HTTPS in prod | process | web | The `!` non-null assertion in `protected/page.tsx:8` will silently break if unset. Validate at boot. |
| 3.4 | Strict CSP, with `connect-src` covering Clerk + the API origin | missing | web | Add via `next.config.ts` `headers()`. Without CSP, a single XSS exfiltrates session tokens. |
| 3.5 | `frame-ancestors 'none'` (or explicit list) | missing | web | Clickjacking. Pair with `X-Frame-Options: DENY` for older clients. |
| 3.6 | `Referrer-Policy: strict-origin-when-cross-origin` | missing | web | Prevents path leakage to third parties. |
| 3.7 | `Permissions-Policy` minimal | missing | web | Disable features the app doesn't use (camera, microphone, geolocation, …). |
| 3.8 | Cookies set by Clerk are `Secure`, `HttpOnly`, `SameSite=Lax` | lab | clerk | Clerk's defaults are correct; verify in DevTools after a real prod login. |
| 3.9 | All fetches that need auth use `cache: "no-store"` or explicit revalidation | lab | web | Already set at `protected/page.tsx:11`. Cached auth responses are an easy data leak. |
| 3.10 | No tokens in URLs, no tokens in error pages | lab | web | Audit `error.tsx` / `not-found.tsx` before launch. |

## 4. Secrets & configuration

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 4.1 | `.env*` files git-ignored | lab | process | `web/.env.example` and `api/.env.example` are the only committed templates. |
| 4.2 | Secrets injected from a managed store, not files on disk | missing | infra | AWS Secrets Manager / SSM / Vault. The lab uses `dotenvy`; do not ship that pattern. |
| 4.3 | Rotation runbook for Clerk publishable / secret keys | missing | process | Required after any suspected compromise or contributor offboarding. Test it once before you need it. |
| 4.4 | Clerk **secret key** never reaches the browser | lab | web | Only `NEXT_PUBLIC_*` is bundled. Audit any new env reads. |
| 4.5 | `CLERK_AUTHORIZED_PARTIES` is the prod origin list, not localhost | process | infra | Forgetting this is the most common "works in staging, breaks in prod" failure for this stack. |
| 4.6 | No secrets in CI logs | process | infra | Mark CI variables as secret; never `echo $TOKEN`. |

## 5. Observability & forensics

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 5.1 | Structured logs (JSON) with `request_id`, `user_id` (`sub`), `route`, `status`, `latency_ms` | partial | api | Replace `tracing_subscriber::fmt::layer()` with a JSON layer in prod and propagate a request id via `tower-http::request_id`. |
| 5.2 | Logs never contain JWTs, JWKS payloads, or PII | lab | api | Audit before adding any `tracing::debug!(headers = ?req.headers())`. |
| 5.3 | Metrics: request rate, error rate, p50/p95/p99 latency, JWKS cache hit ratio | missing | api | Expose `/metrics` via `axum-prometheus` or push to OTLP. Cache hit ratio is the single best signal for JWKS health. |
| 5.4 | Auth failures emit a distinct, alertable metric | missing | api | Spike in 401s = credential-stuffing or a broken client. Spike in 403s = misconfigured `azp` or a real attacker. |
| 5.5 | Tracing context propagated from web → api | missing | both | W3C `traceparent` header. Lets you stitch a slow `/protected` render to a slow `/me` JWKS refresh. |
| 5.6 | Alerts on: 5xx > 1%, 401 surge, JWKS refresh failure, p95 latency, instance OOM | missing | infra | Without alerts, you find out from a customer. |
| 5.7 | Log retention meets compliance (typically 30–90d hot, 1y cold) | process | infra | Whatever your privacy posture requires. Auth logs are sensitive — encrypt at rest. |

## 6. Build, supply chain, runtime

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 6.1 | `cargo audit` / `cargo deny` in CI | missing | process | Catches known CVEs in the dependency tree. |
| 6.2 | `npm audit --omit=dev` in CI; Renovate/Dependabot enabled | missing | process | Same idea for the web side. |
| 6.3 | Release build is `cargo build --release` with `strip = "symbols"` | missing | api | Smaller binary, fewer symbols leaking into stack traces. |
| 6.4 | Container is minimal base (distroless / `gcr.io/distroless/cc`) | missing | infra | Reduces CVE surface vs full Debian. |
| 6.5 | Container runs as non-root with read-only rootfs | missing | infra | Defence-in-depth if a handler is ever exploitable. |
| 6.6 | Image is reproducible and pinned by digest, not tag, in deploy manifests | missing | infra | `:latest` is how you ship the wrong version on a Friday. |
| 6.7 | SBOM emitted and stored alongside the image | missing | infra | `cargo cyclonedx` + `syft` for the container. Required by an increasing number of buyers/regulators. |
| 6.8 | All dependencies use `rustls` (not OpenSSL) | lab | api | `reqwest` is built with `rustls-tls` (`Cargo.toml:12`). Keep it. |

## 7. Network & infrastructure

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 7.1 | API is reachable only from the load balancer (private subnet / SG) | missing | infra | Do not expose port 8080 directly to the internet. |
| 7.2 | WAF in front of the LB (managed rules + IP rate limit) | missing | infra | Cheap blanket protection vs scrapers and basic OWASP categories. |
| 7.3 | DDoS protection (CloudFront / Cloudflare / equivalent) | missing | infra | A single attacker with a botnet can saturate a small instance otherwise. |
| 7.4 | Autoscaling on CPU + request concurrency | missing | infra | Pairs with item 2.3. |
| 7.5 | Multi-AZ deployment for both API and ingress | missing | infra | An AZ outage should degrade, not down, the service. |
| 7.6 | Egress restricted to Clerk JWKS host + observability sinks | missing | infra | Limits exfiltration if a process is compromised. |

## 8. Operational readiness

| # | Item | State | Owner | Why |
|---|---|---|---|---|
| 8.1 | Runbook for "Clerk JWKS is down" | missing | process | Last-good-cache extends, then everyone is logged out. Decide and document the user-facing behaviour. |
| 8.2 | Runbook for "leaked Clerk secret key" | missing | process | Rotate in Clerk dashboard → redeploy with new secret → invalidate sessions if needed. Practise it once. |
| 8.3 | Runbook for "API instance hot-looping on bad config" | missing | process | Configuration validation at boot + clear error in logs + manual rollback path. |
| 8.4 | On-call rotation with paging targets wired to item 5.6 alerts | missing | process | Alerts without owners are noise. |
| 8.5 | Pre-launch smoke (the one in `README.md` §5) is automated and run on every deploy | partial | process | Today it's a manual `curl`. Wrap it in a deploy-gating job. |
| 8.6 | Backup / disaster recovery scope explicitly defined | process | infra | This lab is stateless on the API side; for any real app, document what is stateful and how it is restored. |

## 9. Out of scope for this lab (record the decision, don't pretend)

These are real production concerns but deliberately not solved here. Calling them out makes future ports honest:

- **Refresh-token rotation and revocation lists** — Clerk owns the session lifecycle.
- **Multi-tenancy / org switching** — there is one Clerk app, one issuer, one audience.
- **Authorization / RBAC** — `/me` returns claims; it does not authorise anything. Real apps need a permission layer on top.
- **Audit log of authenticated actions** — `tracing` is for ops, not for compliance.
- **Data plane** (DB, queues, secret rotation cadence, backups) — none of those exist in this repo.

---

## Pre-launch sign-off

Before flipping DNS:

- [ ] Every `missing` and `partial` row above is closed, deferred-with-justification, or moved to §9.
- [ ] §8.5 smoke runs green from a deploy job, not a laptop.
- [ ] §5.6 alerts page a human inside 5 min on a deliberate failure injection.
- [ ] §8.2 rotation runbook executed end-to-end in staging.
- [ ] One engineer who did not write the code has read this doc and signed off.
