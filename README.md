# Next.js + Clerk + Rust/Axum Auth Lab

Reference monorepo showing how a Next.js (App Router) frontend authenticated with **Clerk** obtains a session JWT and calls a **Rust/Axum** backend that verifies the JWT against Clerk's JWKS.

```
┌─────────────┐   sign in    ┌────────┐
│  Browser    │ ───────────► │ Clerk  │
└──────┬──────┘              └────────┘
       │ session JWT
       ▼
┌─────────────┐  Authorization: Bearer <jwt>   ┌────────────────┐
│  Next.js    │ ─────────────────────────────► │  Rust / Axum   │
│  web/       │                                │  api/          │
└─────────────┘                                └───────┬────────┘
                                                       │ JWKS verify
                                                       ▼
                                                ┌────────────┐
                                                │ Clerk JWKS │
                                                └────────────┘
```

- `web/` — Next.js 15 App Router + `@clerk/nextjs`. Sign in/up + a protected page that fetches the Rust API.
- `api/` — Rust + Axum. `/health` (public) and `/me` (protected, returns claims). JWKS-based verification, cached in-memory.
- `docs/` — architecture and AI knowledge map for future agents.

## 1. Prerequisites

- Node 20+ and npm (or pnpm)
- Rust 1.78+ (stable) and Cargo
- A free Clerk account → <https://dashboard.clerk.com>

## 2. Clerk dashboard setup

1. Create a new Clerk application (Email + any social provider you want).
2. Under **API Keys**, copy:
   - **Publishable key** → `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY`
   - **Secret key** → `CLERK_SECRET_KEY`
3. Under **API Keys → Advanced → JWT public key**, copy the **Issuer URL** (something like `https://xxxxxxxx.clerk.accounts.dev`) → `CLERK_JWT_ISSUER`.
4. Under **Sessions → Customize session token** (optional): the default session token is sufficient for this lab — no extra claims required.
5. Under **Domains → Allowed origins**, add `http://localhost:3000` (and any other dev origin).

## 3. Local environment

Copy the templates and fill in real values (these files are gitignored):

```bash
cp web/.env.example web/.env.local
cp api/.env.example api/.env
```

| Variable                              | Where      | Value                                           |
| ------------------------------------- | ---------- | ----------------------------------------------- |
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY`   | `web/`     | from Clerk dashboard                            |
| `CLERK_SECRET_KEY`                    | `web/`     | from Clerk dashboard                            |
| `NEXT_PUBLIC_API_BASE_URL`            | `web/`     | `http://localhost:8080` (the Rust API)          |
| `CLERK_JWT_ISSUER`                    | `api/`     | issuer URL from Clerk dashboard                 |
| `CLERK_AUTHORIZED_PARTIES`            | `api/`     | comma-separated origins, e.g. `http://localhost:3000` |
| `API_PORT`                            | `api/`     | `8080`                                          |

## 4. Run

In two terminals:

```bash
# Terminal A — Rust API
cd api
cargo run

# Terminal B — Next.js web
cd web
npm install
npm run dev
```

Then open <http://localhost:3000>, sign up, and navigate to `/protected`. You should see the JSON claims returned by the Rust `/me` endpoint.

## 5. Smoke test (no browser)

```bash
# Public endpoint
curl -s http://localhost:8080/health
# → {"status":"ok"}

# Protected endpoint without a token
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/me
# → 401

# Protected endpoint with a real Clerk JWT
# Grab a token in the browser console after signing in:
#   await window.Clerk.session.getToken()
TOKEN="paste-jwt-here"
curl -s -H "Authorization: Bearer $TOKEN" http://localhost:8080/me | jq
# → {"sub":"user_...","iss":"...","exp":...,"azp":"http://localhost:3000",...}
```

## 6. Repo layout

```
.
├── README.md
├── docs/
│   ├── architecture.md
│   └── ai-knowledge-map/      # folder — start at README.md
├── web/                    # Next.js 15 + @clerk/nextjs
└── api/                    # Rust + Axum + jsonwebtoken
```

## 7. Where to read next

- `docs/authentication-flow.md` — end-to-end walkthrough of the auth flow from sign-in to verified `/me` response, with per-step file references and a failure-mode matrix.
- `docs/architecture.md` — JWT flow, JWKS caching, scope boundaries.
- `docs/architecture-diagrams.md` — Mermaid renderings of the same flows (system context, sequence, request-verification decision tree, JWKS cache state machine, failure-mode map).
- `docs/backend-bearer-token.md` — focused reference for how the Rust/Axum side accepts and verifies `Authorization: Bearer …`.
- `docs/production-hardening.md` — pre-launch checklist: what this lab does today vs. what a real production deployment needs (auth, API, web, secrets, observability, supply chain, infra, ops).
- `docs/ai-knowledge-map/` — folder of topic-scoped notes for future agents (start at [`docs/ai-knowledge-map/README.md`](./docs/ai-knowledge-map/README.md)). Covers entry points, shared config, gotchas, extension recipes, scope, and related docs.
