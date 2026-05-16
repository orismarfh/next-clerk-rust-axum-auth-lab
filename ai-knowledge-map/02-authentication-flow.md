# Authentication Flow

1. User opens Next.js frontend.
2. User signs in/up with Clerk components.
3. Frontend requests Clerk session JWT using `getToken()`.
4. Frontend sends request to Axum protected route with header:
   - `Authorization: Bearer <jwt>`
5. Backend checks Bearer token presence and format.
6. In production, backend must validate JWT signature/claims against Clerk JWKS.
7. Backend returns protected response only after successful validation.
