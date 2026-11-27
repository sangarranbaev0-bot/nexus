# Threat Model — NexusChat v0.6

## Assets

| Asset | Why it matters |
|---|---|
| User credentials (password hash, email, JWT secret) | Direct account takeover |
| Database records (messages, DMs, tickets) | Privacy leak |
| MinIO S3 bucket (`nexuschat`) | Media/file theft or deletion |
| OAuth client secrets | Third-party app impersonation |
| Server operator identity / admin role | Privilege escalation |

## Key assumptions

- Trust the Postgres server (runs on same host or inside Docker).
- TLS termination is outside `nexus-server`; the binary itself serves plain HTTP by default.
- Rate limits are per-process memory; a restart clears all counters.

## Risks and mitigations

### R1 — Credential brute-force / credential stuffing
**Impact:** Medium · **Likelihood:** High  
PostgreSQL stores password hashes (bcrypt/argon2 — whatever `auth.rs` uses). An unauthenticated API endpoint `/api/auth/login` accepts any email/password pair without CAPTCHA.  
**Mitigations in place:** IP-based rate limit (10 auth attempts / min per key), bcrypt/argon2 hash cost.  
**Remaining:** No lockout after N failures, no IP reputation check. Acceptable for current scale.

### R2 — JWT secret compromise
**Impact:** Critical · **Likelihood:** Low  
If `JWT_SECRET` env var leaks, an attacker can forge arbitrary tokens.  
**Mitigations in place:** Secret never logged; config driven via `.env`.  
**Recommended:** Rotate regularly; store in a secrets manager (Vault, AWS Secrets Manager) in production.

### R3 — SQL injection via raw queries
**Impact:** Critical · **Likelihood:** Low  
`diesel_async` is used everywhere; raw `sql_query!` only appears in the VACUUM ANALYZE task which takes no user input.  
**Status:** Clean. Verify at every PR that adds a new raw query.

### R4 — Server-side request forgery (SSRF)
**Impact:** Medium · **Likelihood:** Medium  
`attachments.rs` fetches remote URLs (`reqwest::get(url)`) when uploading from a URL. A malicious user could point the link at an internal service.  
**Mitigation:** Request has 3-step redirect policy (`limited(3)`), but no URL allowlist exists.  
**Recommended:** Validate URL scheme (`http`, `https`) and reject `file://`, `gopher://` etc.; optional: block private IP ranges.

### R5 — Cross-site scripting (XSS) in markdown
**Impact:** High · **Likelihood:** Low  
Markdown is rendered client-side. The server only stores raw text. If a future feature introduces HTML or unescaped content from the server into a dangerous sink, XSS is possible.  
**Status:** Currently safe — `client/src/Markdown.tsx` handles rendering. Code review required for any new server→client content pipeline.

### R6 — WebSocket origin spoofing
**Impact:** Medium · **Likeliness:** Low  
WSS connections originate from arbitrary browsers. Denial-of-service via many WS connections is trivially achievable.  
**Mitigation:** None currently beyond authentication.  
**Recommended:** Add Origin header validation when served behind a reverse proxy; consider a max-connections-per-user limit in `presence_connections`.

### R7 — DoS via large sticker/attachment uploads
**Impact:** Medium · **Likelihood:** Medium  
No hard size limit documented at the route level (checked by MinIO bucket policy?). A single oversized upload blocks the connection and occupies disk.  
**Recommended:** Add explicit `Content-Length` header check (e.g., reject > 10 MB at Axum middleware) before hitting S3.

### R8 — Rate-limit bypass via IP rotation
**Impact:** Low · **Likelihood:** Medium  
Rate limiter key is derived from the connection identity. Behind NAT or multiple proxies, many users share one key.  
**Mitigation:** Acceptable trade-off. If precise per-user limiting is needed later, use the authenticated `user_id` as key instead of the current key.

### R9 — Plaintext transport (no HSTS)
**Impact:** Medium · **Likelihood:** High (if deployed on plain HTTP)  
Client sends JWTs and cookies over the wire unencrypted if not behind a TLS reverse proxy.  
**Mitigations in place:** Server emits `Strict-Transport-Security` only when `HTTPS_ONLY` env var is set.  
**Recommendation:** Always deploy behind nginx/Caddy/Traefik with TLS; set `HTTPS_ONLY=true` in production.

### R10 — Scheduled message leakage via timing channel
**Impact:** Low · **Likelihood:** Low  
Scheduled messages expose their send time to anyone who can observe `sent_at` timestamps in the channel.  
**Mitigation:** Intended behavior; scheduled messages are normal messages once delivered.  
**Not a concern.**

## Data retention

- **Refresh tokens**: Revoked token rows accumulate indefinitely. Recommend periodic cleanup.
- **Read states**: One row per `(user_id, channel_id)`. Negligible growth.
- **Audit log**: Append-only; grows without bound. Consider partitioning or TTL.

## Open items

- [ ] Add Content-Length guard for attachment/sticker uploads (>10 MB rejection).
- [ ] Add Origin header validation in WebSocket handler.
- [ ] Implement scheduled refresh-token cleanup job.
- [ ] Audit-log TTL or partitioning strategy.
- [ ] Document approved CORS origins for production deployments.
