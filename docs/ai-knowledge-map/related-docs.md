# Related Docs & Delivery History

## Sibling docs in `docs/`

The deep-dives this map points into. Each is self-contained; the map exists so you don't have to read them top-to-bottom.

| Doc | Use it for |
|---|---|
| [`../authentication-flow.md`](../authentication-flow.md) | End-to-end walkthrough — what runs where, in what order, with which data. Includes per-step file references, a failure-mode matrix, and the session-lifetime table. |
| [`../architecture.md`](../architecture.md) | Higher-level decisions: token type, verification strategy, caching, CORS, what's deliberately out of scope. |
| [`../architecture-diagrams.md`](../architecture-diagrams.md) | Mermaid renderings of the same flows — system context, sequence, `require_auth` decision tree, JWKS cache state machine, config topology, failure-mode map. |
| [`../backend-bearer-token.md`](../backend-bearer-token.md) | Backend-only reference: wire contract, status-code matrix, code map (which Rust file owns which step), smoke recipes, extension hooks. |
| [`../production-hardening.md`](../production-hardening.md) | Pre-launch checklist — what this lab does today vs. what a real production deployment needs across auth, API, web, secrets, observability, supply chain, infra, ops. |

## Files in this folder

For navigation, see [`README.md`](./README.md). The full index:

- [`README.md`](./README.md) — TL;DR + folder map
- [`entry-points.md`](./entry-points.md) — file-by-file tour
- [`shared-config.md`](./shared-config.md) — env vars that span `web/` ↔ `api/`
- [`gotchas.md`](./gotchas.md) — symptom → cause map
- [`extension-recipes.md`](./extension-recipes.md) — how to add routes, roles, webhooks, swap caches/providers
- [`scope.md`](./scope.md) — what is intentionally absent
- [`related-docs.md`](./related-docs.md) — this file

## Delivery history

| Issue | What it delivered |
|---|---|
| [`ZIZ-64`](/ZIZ/issues/ZIZ-64) | Parent — Inbox Board entry for the lab. |
| [`ZIZ-65`](/ZIZ/issues/ZIZ-65) | Plan and build the Next.js + Clerk + Rust/Axum reference. |
| [`ZIZ-66`](/ZIZ/issues/ZIZ-66) | Rust/Axum API with Clerk JWKS auth middleware. |
| [`ZIZ-67`](/ZIZ/issues/ZIZ-67) | Next.js + Clerk web with protected page calling the Rust API. |
| [`ZIZ-68`](/ZIZ/issues/ZIZ-68) | Smoke verification + AI knowledge map polish. |
| [`ZIZ-70`](/ZIZ/issues/ZIZ-70) | Bearer-token reference doc (`backend-bearer-token.md`). |
| [`ZIZ-71`](/ZIZ/issues/ZIZ-71) | End-to-end auth-flow doc (`authentication-flow.md`). |
| [`ZIZ-72`](/ZIZ/issues/ZIZ-72) | Mermaid architecture diagrams (`architecture-diagrams.md`). |
| [`ZIZ-73`](/ZIZ/issues/ZIZ-73) | Split the single `ai-knowledge-map.md` into this folder. |
| [`ZIZ-74`](/ZIZ/issues/ZIZ-74) | Production hardening checklist (`production-hardening.md`). |
