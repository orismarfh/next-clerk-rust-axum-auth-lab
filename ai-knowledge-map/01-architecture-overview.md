# Architecture Overview

## Frontend (`/frontend`)
- Next.js App Router project.
- Clerk is initialized in `app/layout.tsx` via `ClerkProvider`.
- `app/auth-lab.tsx` demonstrates:
  - Sign in/sign up UI
  - Retrieving a Clerk JWT (`getToken()`)
  - Calling Axum public/protected routes with and without Bearer auth

## Backend (`/backend`)
- Rust + Axum service.
- Public route: `GET /api/public`.
- Protected route: `GET /api/protected`.
- Protected route checks for `Authorization: Bearer <token>`.
- Code comments indicate where real Clerk JWT verification should be performed.
