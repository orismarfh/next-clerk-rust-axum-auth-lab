# AI Knowledge Map

> Purpose: orient a future agent quickly so they can extend this lab without re-reading every file. This `README.md` is the **only** doc you must read before opening source — everything else in this folder is a deep-dive on one slice.

## TL;DR

- Two apps: `web/` (Next.js + Clerk) and `api/` (Rust + Axum).
- The frontend gets a JWT from Clerk (`getToken()`), sends it as `Authorization: Bearer …`, and the Rust API verifies it locally against Clerk's JWKS.
- All secrets live in per-app `.env` files (gitignored). Templates: `web/.env.example`, `api/.env.example`.

## Map of this folder

| Open this when… | File |
|---|---|
| You need to know which source file to open for a concern | [`entry-points.md`](./entry-points.md) |
| You're touching env vars or wiring config across `web/` ↔ `api/` | [`shared-config.md`](./shared-config.md) |
| Auth "almost works" but something is off | [`gotchas.md`](./gotchas.md) |
| You're adding a new protected route, a role check, a webhook, etc. | [`extension-recipes.md`](./extension-recipes.md) |
| You're tempted to add a DB / CI / tests / UI library and want to know why none exist | [`scope.md`](./scope.md) |
| You want the wider doc set or the delivery history | [`related-docs.md`](./related-docs.md) |

## Sibling docs (outside this folder)

These are the deep references the map points into. You usually don't read them top-to-bottom — `entry-points.md` and `gotchas.md` will tell you which section of which one you need.

| Concern | File |
|---|---|
| End-to-end auth flow (sign-in → verified `/me`) | [`../authentication-flow.md`](../authentication-flow.md) |
| High-level architecture + design decisions | [`../architecture.md`](../architecture.md) |
| Mermaid diagrams (system context, sequence, decision tree, JWKS state machine) | [`../architecture-diagrams.md`](../architecture-diagrams.md) |
| Backend bearer-token drill-down (wire contract, status codes, code map) | [`../backend-bearer-token.md`](../backend-bearer-token.md) |
| Production hardening checklist (what's lab-grade vs. prod) | [`../production-hardening.md`](../production-hardening.md) |

## How to use this map

1. Read this `README.md` (you're here).
2. Skim [`entry-points.md`](./entry-points.md) so you know which source file owns each concern.
3. Open the deep-dive that matches your task — usually [`gotchas.md`](./gotchas.md) for debugging, [`extension-recipes.md`](./extension-recipes.md) for building.
4. Only then drop into source.

If you find yourself answering the same "where does X live?" question twice, that's a signal — update the relevant file here so the next agent doesn't repeat the search.
