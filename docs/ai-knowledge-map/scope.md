# Scope (what's intentionally missing)

This lab is deliberately small. The list below exists so the next agent doesn't waste time adding something that was already considered and dropped.

For the production-side companion view (what you'd *have to* add before shipping), see [`../production-hardening.md`](../production-hardening.md).

## Not in this lab — and why

| Missing | Why it's absent |
|---|---|
| **Database / ORM / migrations** | The whole point is to show that Clerk-backed auth needs no per-request DB lookup. Adding a DB would imply a user mirror, which is a separate concern (covered as a recipe in [`extension-recipes.md`](./extension-recipes.md), not as a default). |
| **Clerk webhooks (`user.created` etc.)** | Webhooks are a *consequence* of having a DB. Without one, there's nothing to write to. The recipe is in [`extension-recipes.md`](./extension-recipes.md). |
| **CI / GitHub Actions** | Adds noise to a reference repo. The smoke test in [`../../README.md`](../../README.md) is two `curl`s and is the canonical verification. |
| **Dockerfile / compose** | Same reason. Adding it would couple the lab to an opinion about deployment that the reader may not share. |
| **Tests beyond the smoke** | A test suite would dominate the line count and obscure the ~200 lines of auth code that are the actual deliverable. The smoke is in [`../../README.md`](../../README.md). |
| **Tailwind / UI library** | The protected page renders raw JSON on purpose — the goal is to make the auth boundary obvious. Styling would dilute that signal. |
| **Refresh-token plumbing** | The Clerk SDK handles refresh transparently. There is no code we'd write here that wouldn't be undone by the SDK. |
| **Role / permission system** | Reading claims is shown. A full RBAC system would expand scope past "show me how Bearer auth works." A starter recipe is in [`extension-recipes.md`](./extension-recipes.md). |
| **Cookie-based session forwarding** | The API intentionally accepts only `Authorization: Bearer …` to model how a third-party caller (mobile app, partner) would use it. Cookies would add CSRF surface for no win in this lab. |

## When the absence becomes wrong

If you're about to extend this lab and find yourself reaching for one of the above, ask: am I still building the *reference* (i.e. this stays in the repo), or am I forking it into a real product?

- **Reference**: keep it absent. Document the extension as a recipe instead.
- **Real product**: fork the repo, and the [`../production-hardening.md`](../production-hardening.md) checklist is your starting point.

The line in the sand: anything that would force a future reader to wade through infrastructure to find the ~200 lines of auth code does not belong here.
