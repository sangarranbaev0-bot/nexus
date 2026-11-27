# TODO — Текущие задачи (видим постоянно)

## Стабильность / баги
- [x] `group_dms` — убрать 429 при параллельном запуске: добавить `rate_limit::reset()` в `common::call`; `cargo test --test groups` — 3/3 ✓
- [x] Повторно упаковать образ Docker: `docker compose --profile app up -d --build --force-recreate app` — ✓ (health OK)
- [x] Удалить/перезаписать старый asset `index-kiW2Y_gH.js` в Docker (permission denied; старый не используется, используется `index-GvhQQy9w.js`)
- [x] Dockerfile: node:22-slim, rust:1.89-slim, debian:trixie-slim, исправить pnpm corepack и healthcheck minio через mc

## Discord-like функции (без постоянного сервера/новых сервисов)
- [x] Streaming/Screen share: WebRTC P2P в голосовых/stage-каналах (`voice.rs`), UI для стримов/экрана (`VoiceView.tsx`).
- [x] Nitro-like UX: цвета ролей + бордеры (`Dialogs.tsx`); кастомные эмодзи + паки-категории (`EmojiPicker.tsx`, `emojis.rs`); остальное: курсоры/аватарки, баджи (`UserSettings.tsx`, `AuthPage.tsx`)
- [x] Boost/подписки: уровни сервера, бусты, quests, monetization — UI + лимиты, без оплаты пока.
- [x] Apps/Bots/Integrations: OAuth-приложения + вебхуки + боты (`auth.rs`, `servers.rs`), Discord API-совместимый упрощённый shim.
- [x] Messages: mentions, custom emojis, stock, scheduled messages, drafts, links (`messages.ts`, `ChatView.tsx`).
- [x] Moderation: bans/kicks/timeouts, audit log with action labels, member management UI (`servers.rs`, `Settings.tsx`).
- [x] Mobile push: manifest/service worker + иконки; позже push-subscription и история уведомлений

## Архитектура / производительность
- [x] Сервер: connection pool tuning, graceful shutdown, `/health`, retries (`pool.rs`, `lib.rs`).
- [x] Клиент: code splitting, image preloading, HTTP cache.
- [x] База: background VACUUM ANALYZE task.

## Тесты / качество
- [x] Интеграционные тесты для tickets; rate limit через HTTP (`tests/tickets.rs`, `tests/rate_limit.rs`).
- [ ] E2E для UI (тема, тикеты, каналы, мультиаккаунт).
- [x] A11y: ARIA labels, focus management (<dialog> native), reduced motion, messages aria-live.
- [x] Docs: updated README with v0.6 features list; added THREAT_MODEL.md.

## Docker / deployment
- [x] Multi-stage Docker с кешированием (`--cache-from`, BuildKit), health checks, rolling deploy, canary.

## Security / hygiene
- [x] 2FA done; OAuth/state PKCE; audit; security headers (CSP, HSTS); RBAC.
- [x] Threat model (docs/THREAT_MODEL.md); DB backup script (scripts/backup_db.ps1); encryption at rest via Postgres/MinIO + OS layer documented.
