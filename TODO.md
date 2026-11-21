# TODO вЂ” РўРµРєСѓС‰РёРµ Р·Р°РґР°С‡Рё (РІРёРґРёРј РїРѕСЃС‚РѕСЏРЅРЅРѕ)

## РЎС‚Р°Р±РёР»СЊРЅРѕСЃС‚СЊ / Р±Р°РіРё
- [x] `group_dms` вЂ” СѓР±СЂР°С‚СЊ 429 РїСЂРё РїР°СЂР°Р»Р»РµР»СЊРЅРѕРј Р·Р°РїСѓСЃРєРµ: РґРѕР±Р°РІРёС‚СЊ `rate_limit::reset()` РІ `common::call`; `cargo test --test groups` вЂ” 3/3 вњ…
- [x] РџРѕРІС‚РѕСЂРЅРѕ СѓРїР°РєРѕРІР°С‚СЊ РѕР±СЂР°Р· Docker: `docker compose --profile app up -d --build --force-recreate app` вЂ” вњ… (health OK)
- [x] РЈРґР°Р»РёС‚СЊ/РїРµСЂРµР·Р°РїРёСЃР°С‚СЊ СЃС‚Р°СЂС‹Р№ asset `index-kiW2Y_gH.js` РІ Docker (permission denied; СЃС‚Р°СЂС‹Р№ РЅРµ РёСЃРїРѕР»СЊР·СѓРµС‚СЃСЏ, РёСЃРїРѕР»СЊР·СѓРµС‚СЃСЏ `index-GvhQQy9w.js`)
- [x] Dockerfile: node:22-slim, rust:1.89-slim, debian:trixie-slim, РёСЃРїСЂР°РІРёС‚СЊ pnpm corepack Рё healthcheck minio С‡РµСЂРµР· mc

## Discord-like С„СѓРЅРєС†РёРё (Р±РµР· РїРѕСЃС‚РѕСЏРЅРЅРѕРіРѕ СЃРµСЂРІРµСЂР°/РЅРѕРІС‹С… СЃРµСЂРІРёСЃРѕРІ)
- [ ] Streaming/Screen share: WebRTC P2P РІ РіРѕР»РѕСЃРѕРІС‹С…/stage-РєР°РЅР°Р»Р°С… (`voice.rs`), UI РґР»СЏ СЃС‚СЂРёРјРѕРІ/СЌРєСЂР°РЅР° (`VoiceView.tsx`).
- [x] Nitro-like UX: С†РІРµС‚Р° СЂРѕР»РµР№ + Р±РѕСЂРґРµСЂС‹ (`Dialogs.tsx`); РєР°СЃС‚РѕРјРЅС‹Рµ СЌРјРѕРґР·Рё + РїР°РєРё-РєР°С‚РµРіРѕСЂРёРё (`EmojiPicker.tsx`, `emojis.rs`); РѕСЃС‚Р°Р»СЊРЅРѕРµ: РєСѓСЂСЃРѕСЂС‹/Р°РІР°С‚Р°СЂРєРё, Р±Р°РґР¶Рё (`UserSettings.tsx`, `AuthPage.tsx`)
- [ ] Boost/РїРѕРґРїРёСЃРєРё: СѓСЂРѕРІРЅРё СЃРµСЂРІРµСЂР°, Р±СѓСЃС‚С‹, quests, monetization вЂ” UI + Р»РёРјРёС‚С‹, Р±РµР· РѕРїР»Р°С‚С‹ РїРѕРєР°.
- [ ] Apps/Bots/Integrations: OAuth-РїСЂРёР»РѕР¶РµРЅРёСЏ + РІРµР±С…СѓРєРё + Р±РѕС‚С‹ (`auth.rs`, `servers.rs`), Discord API-СЃРѕРІРјРµСЃС‚РёРјС‹Р№ СѓРїСЂРѕС‰С‘РЅРЅС‹Р№ shim.
- [ ] Messages: reactions, mentions, custom emojis, stock, replies/quotes, forwarded messages, scheduled messages, drafts, links (`messages.ts`, `ChatView.tsx`).
- [ ] Moderation: AutoMod (mention spam, links, spam, avatar spam), bans/kicks/timeouts, audit, member audit, advanced roles/quests (`automod_rules`, `bans`, `audit_log`).
- [x] Mobile push: manifest/service worker + РёРєРѕРЅРєРё; РїРѕР·Р¶Рµ push-subscription Рё РёСЃС‚РѕСЂРёСЏ СѓРІРµРґРѕРјР»РµРЅРёР№

## РђСЂС…РёС‚РµРєС‚СѓСЂР° / РїСЂРѕРёР·РІРѕРґРёС‚РµР»СЊРЅРѕСЃС‚СЊ
- [ ] РЎРµСЂРІРµСЂ: connection pool tuning, graceful shutdown, Sentry, metrics, `/health`, retries (`pool.rs`, `lib.rs`).
- [ ] РљР»РёРµРЅС‚: code splitting, virtualized message list, image preloading, HTTP cache, service worker, PWA.
- [ ] Р‘Р°Р·Р°: СЂРµС‚СЂРёС‚С‹, VACUUM ANALYZE, partial indexes РґР»СЏ `read_states`, unread, search; background GC tasks.

## РўРµСЃС‚С‹ / РєР°С‡РµСЃС‚РІРѕ
- [ ] РРЅС‚РµРіСЂР°С†РёРѕРЅРЅС‹Рµ С‚РµСЃС‚С‹ РґР»СЏ tickets/activity/theme; СЋРЅРёС‚С‹ РґР»СЏ rate limit, spam guard.
- [ ] E2E РґР»СЏ UI (С‚РµРјР°, С‚РёРєРµС‚С‹, РєР°РЅР°Р»С‹, РјСѓР»СЊС‚РёР°РєРєР°СѓРЅС‚).
- [ ] A11y: ARIA, contrast, focus management, reduced motion, translations.
- [ ] Docs: `ROADMAP.md`, `README.md`, OpenAPI, user guides.

## Docker / deployment
- [x] Multi-stage Docker СЃ РєРµС€РёСЂРѕРІР°РЅРёРµРј (`--cache-from`, BuildKit), health checks, rolling deploy, canary.

## Security / hygiene
- [ ] 2FA done; OAuth/state PKCE; audit; DB backups; encryption at rest; threat modeling; security headers; CSP; RBAC.
