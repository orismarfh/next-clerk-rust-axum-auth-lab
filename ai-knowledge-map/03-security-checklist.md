# Security Checklist

- [ ] Never trust token presence alone in production.
- [ ] Validate JWT signature against Clerk JWKS.
- [ ] Validate `iss`, `aud`, `exp`, and expected subject claims.
- [ ] Enforce HTTPS for frontend-backend communication.
- [ ] Keep Clerk keys in environment variables, never hardcoded.
- [ ] Add rate limiting and logging before production rollout.
