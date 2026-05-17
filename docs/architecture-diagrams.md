# Architecture Diagrams

Mermaid renderings of the lab's architecture and auth flow. GitHub, GitLab, VS Code, Obsidian, and most other modern Markdown viewers render these blocks inline — no external tooling needed.

Pair this with the prose docs:

- [`architecture.md`](./architecture.md) — decisions and out-of-scope.
- [`authentication-flow.md`](./authentication-flow.md) — narrative walkthrough with file/line references.
- [`backend-bearer-token.md`](./backend-bearer-token.md) — backend-only reference.
- [`ai-knowledge-map.md`](./ai-knowledge-map.md) — file tour and gotchas.

## 1. System context (component view)

Who talks to whom, and over what.

```mermaid
flowchart LR
    Browser["Browser<br/>(user's machine)"]
    Clerk["Clerk<br/>*.clerk.accounts.dev"]
    Web["Next.js — web/<br/>localhost:3000<br/>App Router + clerkMiddleware"]
    Api["Rust / Axum — api/<br/>localhost:8080<br/>JWKS verify + /health + /me"]
    Jwks[("Clerk JWKS<br/>${ISSUER}/.well-known/jwks.json")]

    Browser -- "1. interactive sign-in" --> Clerk
    Clerk -- "2. session cookie + JS bridge" --> Browser
    Browser -- "3. GET /protected (cookie)" --> Web
    Web -- "4. auth().getToken() → RS256 JWT" --> Clerk
    Web -- "5. Authorization: Bearer <jwt>" --> Api
    Api -- "6. on cache miss / TTL: fetch JWKS" --> Jwks
    Jwks -- "RSA public keys (by kid)" --> Api
    Api -- "7. 200 JSON claims" --> Web
    Web -- "8. HTML" --> Browser

    classDef external fill:#fef3c7,stroke:#b45309,color:#7c2d12;
    classDef ours fill:#dbeafe,stroke:#1d4ed8,color:#1e3a8a;
    class Clerk,Jwks external;
    class Web,Api ours;
```

Yellow boxes are out-of-repo (Clerk-hosted). Blue boxes live in this monorepo.

## 2. End-to-end sequence (happy path)

The full sign-in → verified `/me` flow. Mirrors the ASCII sequence in [`authentication-flow.md`](./authentication-flow.md#sequence-happy-path) with the same step numbering.

```mermaid
sequenceDiagram
    autonumber
    actor Browser
    participant Web as Next.js (web/)
    participant Clerk
    participant Api as Rust API (api/)
    participant Jwks as Clerk JWKS

    Browser->>Web: GET /protected
    Web->>Web: clerkMiddleware → auth.protect()<br/>(no session cookie)
    Web-->>Browser: 307 → /sign-in
    Browser->>Clerk: sign in (Clerk-hosted UI)
    Clerk-->>Browser: __session cookie + JS bridge
    Browser->>Web: GET /protected (with __session)
    Web->>Web: clerkMiddleware: auth.protect() passes
    Web->>Clerk: auth().getToken() — mint short-lived RS256 JWT
    Clerk-->>Web: JWT (≈60s TTL)
    Web->>Api: GET /me<br/>Authorization: Bearer <jwt>
    Api->>Api: extract_bearer_token()<br/>decode_header → kid
    alt kid not in cache OR cache TTL expired
        Api->>Jwks: GET /.well-known/jwks.json
        Jwks-->>Api: { keys: [...] }
        Api->>Api: rebuild cache, retry lookup
    end
    Api->>Api: jsonwebtoken::decode<br/>verify RS256 + iss + exp + nbf
    Api->>Api: check azp ∈ CLERK_AUTHORIZED_PARTIES
    Api->>Api: insert Claims into req extensions
    Api-->>Web: 200 { sub, iss, exp, azp, ... }
    Web-->>Browser: HTML (renders claims)
```

## 3. Request verification — decision flow

What happens inside `require_auth` for a single request to a protected route. Each terminal node is a real status code the API returns.

```mermaid
flowchart TD
    Start(["Request → protected route"]) --> H{Authorization header<br/>starts with Bearer/bearer?}
    H -- no --> E401Missing["401 unauthorized<br/>missing Bearer token"]
    H -- yes --> DecodeH{decode_header → kid present?}
    DecodeH -- no --> E401NoKid["401 unauthorized<br/>JWT header missing kid"]
    DecodeH -- yes --> Cache{JwksCache: kid present<br/>AND cache age < 5 min?}
    Cache -- yes --> Verify
    Cache -- no --> Refresh["fetch ${ISSUER}/.well-known/jwks.json"]
    Refresh -- non-2xx or network error --> E500Jwks["500 internal_error<br/>JWKS endpoint returned ..."]
    Refresh -- 2xx --> RetryKid{kid present after refresh?}
    RetryKid -- no --> E401Kid["401 unauthorized<br/>unknown key id"]
    RetryKid -- yes --> Verify["jsonwebtoken::decode<br/>RS256 + iss allowlist<br/>+ exp/nbf window"]
    Verify -- fail --> E401Verify["401 unauthorized<br/>JWT verification failed"]
    Verify -- ok --> Azp{azp ∈ CLERK_AUTHORIZED_PARTIES?}
    Azp -- no / absent --> E403["403 forbidden<br/>azp not in allowlist"]
    Azp -- yes --> Handler["inject Claims into req.extensions<br/>call next layer → handler runs"]
    Handler --> OK["200 OK<br/>(handler's response)"]

    classDef ok fill:#dcfce7,stroke:#15803d,color:#14532d;
    classDef err fill:#fee2e2,stroke:#b91c1c,color:#7f1d1d;
    classDef warn fill:#fef9c3,stroke:#a16207,color:#713f12;
    class OK,Handler ok;
    class E401Missing,E401NoKid,E401Kid,E401Verify,E403 err;
    class E500Jwks warn;
```

Code references: `api/src/auth.rs:216-235` (`require_auth`), `api/src/auth.rs:171-208` (`verify_token`), `api/src/auth.rs:104-127` (`JwksCache::get_or_refresh`).

## 4. JWKS cache state machine

The cache has three observable states from a request's point of view. `JWKS_TTL` is 5 minutes (`auth.rs:81`).

```mermaid
stateDiagram-v2
    [*] --> Empty: process start
    Empty --> Fresh: first request triggers refresh()<br/>fetch JWKS, store fetched_at = now
    Fresh --> Fresh: request with known kid<br/>(cache age < TTL)
    Fresh --> Refreshing: TTL expired OR kid not in cache
    Refreshing --> Fresh: 2xx JWKS, rebuild map<br/>fetched_at := now
    Refreshing --> Empty: JWKS non-2xx / network error<br/>→ 500 to caller (cache untouched)
    Fresh --> Fresh: kid known after refresh
    Refreshing --> Fresh: kid still missing → 401 to caller<br/>(cache replaced with new keyset)
```

> Note: a failed refresh does **not** poison the cache — the previously fresh keys remain usable until the next attempt. A `kid` miss after a successful refresh returns `401 unknown key id`, not 500.

## 5. Configuration topology

Which env var feeds which subsystem, and where the two sides must agree.

```mermaid
flowchart LR
    subgraph Web_env [web/.env.local]
        W1[NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY]
        W2[CLERK_SECRET_KEY]
        W3[NEXT_PUBLIC_API_BASE_URL]
    end

    subgraph Api_env [api/.env]
        A1[CLERK_JWT_ISSUER]
        A2[CLERK_AUTHORIZED_PARTIES]
        A3[API_PORT]
    end

    subgraph Web_runtime [Next.js runtime]
        WClerk[ClerkProvider / clerkMiddleware]
        WFetch["fetch(API_BASE/me)"]
    end

    subgraph Api_runtime [Axum runtime]
        AJwks["JwksCache<br/>(issuer → JWKS URL)"]
        AVerify["verify_token<br/>(iss + azp checks)"]
        ACors["CorsLayer<br/>(allow_origin)"]
        APort[bind 0.0.0.0:API_PORT]
    end

    W1 --> WClerk
    W2 --> WClerk
    W3 --> WFetch
    A1 --> AJwks
    A1 --> AVerify
    A2 --> AVerify
    A2 --> ACors
    A3 --> APort

    WFetch -. "must equal http://host:API_PORT" .-> APort
    WClerk -. "web origin must ∈ allowlist" .-> ACors
    WClerk -. "same Clerk app ⇒ same issuer" .-> AJwks
```

The dotted edges are the cross-app invariants. If any of them breaks, you get the failure modes documented in [`authentication-flow.md`](./authentication-flow.md#failure-modes-status-code-matrix).

## 6. Failure-mode map

Same matrix as the prose docs, rendered as a one-glance flow. Use this when triaging "auth not working".

```mermaid
flowchart LR
    Symptom(["Symptom"]) --> S1{Browser sees<br/>307 → /sign-in}
    S1 -- yes --> R1["expected: no Clerk session<br/>sign in, retry"]
    Symptom --> S2{/me returns 401<br/>missing Bearer token}
    S2 -- yes --> R2["frontend forgot the header<br/>check fetch() options"]
    Symptom --> S3{/me returns 401<br/>JWT verification failed}
    S3 -- yes --> R3["token expired, wrong iss,<br/>or signed by a different Clerk app"]
    Symptom --> S4{/me returns 401<br/>unknown key id}
    S4 -- yes --> R4["Clerk rotated keys faster than<br/>your refresh — check JwksCache"]
    Symptom --> S5{/me returns 403<br/>azp not in allowlist}
    S5 -- yes --> R5["web origin missing from<br/>CLERK_AUTHORIZED_PARTIES"]
    Symptom --> S6{/me returns 500<br/>JWKS endpoint returned ...}
    S6 -- yes --> R6["CLERK_JWT_ISSUER typo or<br/>placeholder — check RUST_LOG=debug"]
    Symptom --> S7{Browser blocks request<br/>CORS error in devtools}
    S7 -- yes --> R7["origin not in CorsLayer allowlist<br/>(same env var as azp)"]

    classDef sym fill:#e0e7ff,stroke:#4338ca,color:#1e1b4b;
    classDef fix fill:#dcfce7,stroke:#15803d,color:#14532d;
    class S1,S2,S3,S4,S5,S6,S7 sym;
    class R1,R2,R3,R4,R5,R6,R7 fix;
```

## Editing tips

- Keep the prose docs as the source of truth for *why*; diagrams are for *shape*.
- When you change a behaviour (e.g. cache TTL, a new failure mode), update the relevant diagram **and** the prose. The repo has no automation that catches drift.
- Mermaid blocks render natively on GitHub. To preview locally, paste into <https://mermaid.live> or open the file in VS Code with the Markdown Preview Mermaid Support extension.

## Related issues

- [ZIZ-72](/ZIZ/issues/ZIZ-72) — this delivery (Mermaid architecture diagrams)
- [ZIZ-71](/ZIZ/issues/ZIZ-71) — end-to-end auth-flow doc (`authentication-flow.md`)
- [ZIZ-70](/ZIZ/issues/ZIZ-70) — backend bearer-token reference (`backend-bearer-token.md`)
