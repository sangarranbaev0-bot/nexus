# NexusChat

Аналог Discord. Бэкенд на Rust (Axum + Tokio + Diesel), фронтенд на React + TypeScript.

```
server/   Rust: HTTP API, WebSocket /ws, миграции Diesel
client/   React + Vite + Tailwind + Zustand + React Router
desktop/  Tauri 2: приложение для Windows с установщиком и автообновлением
```

## Требования

- Rust 1.85+ (edition 2024)
- Node.js 22+ и pnpm
- Docker (PostgreSQL и MinIO) или свои PostgreSQL 16 и S3-совместимое хранилище

## Запуск в разработке

```bash
cp .env.example .env
docker compose up -d postgres minio minio-init   # minio-init создаёт бакет и завершается

# терминал 1: сервер на :3001, миграции применяются при старте
cd server && cargo run

# терминал 2: клиент на :5173, /api и /ws проксируются на сервер
cd client && pnpm install && pnpm dev
```

Откройте http://localhost:5173. Внизу списка каналов должно быть «В сети» и задержка.

## Контракт клиент ↔ сервер

Типы описаны в `server/src/protocol.rs`. TS-версии в `client/src/generated/`
генерирует ts-rs при `cargo test`, руками их не править.

WebSocket `/ws` обменивается JSON вида `{ "type": "...", ... }`:

Первым сообщением клиент обязан прислать `auth` с access-токеном, иначе сервер
ответит `error: unauthorized` и закроет соединение.

| Направление | type    | Поля                    |
| ----------- | ------- | ----------------------- |
| клиент →    | `auth`  | `token`                 |
| сервер →    | `hello` | `connId`, `userId`      |
| клиент →    | `ping`  | `sentAt`                |
| сервер →    | `pong`  | `sentAt`, `serverTime`  |
| сервер →    | `error` | `message`               |
| клиент →    | `typing` | `channelId`            |
| сервер →    | `typing` | `channelId`, `userId`, `username` |
| сервер →    | `messageCreated` / `messageUpdated` | `message` |
| сервер →    | `messageDeleted` | `channelId`, `messageId` |
| сервер →    | `channelsChanged` | `serverId`     |
| клиент →    | `setIdle` | `idle`                 |
| сервер →    | `presenceChanged` | `userId`, `status` (`online` / `idle` / `offline`) |
| сервер →    | `channelRead` | `serverId`, `channelId` (только другим вкладкам того же пользователя) |

`GET /api/health` → `200 { db: true, serverTime }` или `503`, если БД недоступна.

## Аутентификация

| Метод | Путь | Что делает |
| ----- | ---- | ---------- |
| POST | `/api/auth/register` | `{email, username, password}` → `201 {accessToken, user}` |
| POST | `/api/auth/login` | `{email, password}` → `{accessToken, user}` |
| POST | `/api/auth/refresh` | по cookie → новая пара, старый refresh отзывается |
| POST | `/api/auth/logout` | отзывает refresh, удаляет cookie → `204` |
| GET | `/api/me` | `Authorization: Bearer <access>` → `user` |

Access JWT живёт 15 минут и хранится только в памяти вкладки. Refresh живёт 30 дней,
лежит в cookie `httpOnly; Secure; SameSite=Strict; Path=/api/auth`, в БД хранится его SHA-256.
Пароли хэшируются bcrypt (cost 12). `JWT_SECRET` в `.env` обязателен, не короче 32 символов.

## Серверы, каналы, роли

Роли: `owner` > `admin` > `member`. Не-участнику сервера API отвечает `404`
(не раскрываем, что сервер существует), участнику без прав — `403`.

| Метод | Путь | Кто может |
| ----- | ---- | --------- |
| GET / POST | `/api/servers` | свои серверы / создать (создатель — owner, канал `#общий`) |
| GET | `/api/servers/{id}/channels` | участник |
| POST | `/api/servers/{id}/channels` | admin+; имя нормализуется: `Мой Канал` → `мой-канал` |
| DELETE | `/api/channels/{id}` | admin+ |
| GET | `/api/servers/{id}/members` | участник |
| PATCH | `/api/servers/{id}/members/{userId}` | owner; `{role: "admin" \| "member"}` |
| POST | `/api/servers/{id}/invites` | участник; `{expiresInHours?, maxUses?}` → код из 8 символов |
| POST | `/api/invites/{code}/join` | любой; `410`, если истёк или исчерпан |

## Сообщения

| Метод | Путь | Кто может |
| ----- | ---- | --------- |
| GET | `/api/channels/{id}/messages?before=<id>&limit=50` | участник; старше `before`, по возрастанию id, `limit` ≤ 100 |
| POST | `/api/channels/{id}/messages` | участник; `{content}` 1–2000 символов |
| PATCH | `/api/messages/{id}` | только автор |
| DELETE | `/api/messages/{id}` | автор или admin+ |

## События между экземплярами (LISTEN/NOTIFY)

Обработчик пишет в БД и вызывает `pg_notify('nexus_events', …)` с id объекта (не с текстом:
лимит payload — 8000 байт). Каждый экземпляр держит отдельное соединение с `LISTEN nexus_events`,
дочитывает сообщение из БД и раздаёт событие своим WebSocket-подключениям — только участникам
сервера. Поэтому можно запускать несколько экземпляров за балансировщиком без Redis; тест
`events_cross_instances_and_respect_membership` проверяет доставку между двумя экземплярами.

## Непрочитанные и присутствие

`unreadCount` приходит в `GET /api/servers` и `GET /api/servers/{id}/channels`: чужие сообщения
после отметки прочтения и после вступления на сервер, максимум 100 на канал (клиент пишет «99+»).
`POST /api/channels/{id}/read {messageId}` двигает отметку только вперёд.

Присутствие общее для всех экземпляров: каждое WebSocket-подключение — строка
`presence_connections` с пульсом раз в 30 с. online — есть живое подключение не в idle,
idle — все живые подключения бездействуют больше 5 минут, offline — живых нет. Строки без пульса
дольше 90 с (упавший экземпляр) убирает фоновая уборка на любом экземпляре.

## Личные сообщения

ЛС — канал без сервера (`channels.server_id IS NULL`) на двоих (`dm_participants`), один на пару
(`dm_key = "меньший_id:больший_id"`). Поэтому лента, правка, удаление, «печатает…», непрочитанные
и рассылка работают через те же эндпоинты `/api/channels/{id}/…`; события ЛС получают только
двое собеседников. Писать можно любому пользователю по имени. В ЛС нет модераторов: удалить
можно только своё сообщение.

| Метод | Путь | Что делает |
| ----- | ---- | ---------- |
| GET | `/api/dms` | мои диалоги, свежие сверху, со статусом собеседника и `unreadCount` |
| POST | `/api/dms` | `{username}` → открыть диалог или вернуть существующий |

## Файлы

Файлы лежат в S3-совместимом хранилище (локально MinIO, в продакшене любой S3), настройки —
`S3_*` в `.env`. Поэтому файл, загруженный через один экземпляр сервера, отдаётся любым другим.

| Метод | Путь | Что делает |
| ----- | ---- | ---------- |
| POST | `/api/channels/{id}/attachments` | multipart, поле `file`, до 10 МБ → `201 Attachment` |
| POST | `/api/channels/{id}/messages` | `{content, attachmentIds}`; текст можно не писать, если есть файлы (до 10) |
| GET | `/api/attachments/{id}/{имя}?exp=…&sig=…` | скачать по подписанной ссылке из сообщения |

- Тип определяется по первым байтам файла (png, jpg, gif, webp, pdf, zip), а не по имени:
  переименованный `.exe` или SVG со скриптом отклоняются.
- `<img src>` не умеет слать токен, поэтому доступ проверяется, когда ссылку выдают вместе с
  сообщением, а сама ссылка подписана HMAC (ключ выводится из `JWT_SECRET`) и живёт 1–2 суток.
- Ответ: `nosniff`, `Content-Security-Policy: sandbox`, картинки `inline`, остальное `attachment`.
- Удаление сообщения удаляет его файлы; файлы удалённых каналов и неотправленные загрузки
  старше суток убирает фоновая уборка раз в час.

## Мобильные

До 768 px на экране только чат: серверы и каналы выезжают по ☰ и закрываются выбором канала,
касанием фона или Esc. Действия с сообщением открываются касанием (сообщение фокусируемое).
Высота — `100dvh`, поэтому поле ввода не уходит под экранную клавиатуру.

## Свой ПК как сервер (на время разработки)

Нужен только [Docker Desktop](https://www.docker.com/products/docker-desktop/) — Rust и Node ставить не надо.

```bash
cd nexuschat
docker compose --profile app up -d --build   # первый раз 5–10 минут: собирается Rust
```

NexusChat работает на http://localhost:3001 (база, файлы и сам чат в Docker). Сервер слушает
только этот компьютер. Остановить — `docker compose --profile app down`, данные сохраняются;
после изменений в коде — та же команда `up -d --build`.

Приложение для Windows с таким сервером: в настройках репозитория задайте
`NEXUS_SERVER_URL` = `http://localhost:3001` (см. «Приложение для Windows»). Оно откроется,
только когда Docker с NexusChat запущен. Когда переедете на VPS, поменяйте переменную на
`https://…` и выпустите новый тег — установленные приложения обновятся сами.

## Деплой на VPS

Нужны: VPS с Ubuntu 22.04+ или Debian 12 (от 2 ГБ памяти, 20 ГБ диска) и домен,
у которого A-запись указывает на IP сервера.

```bash
git clone https://github.com/OWNER/REPO.git nexuschat && cd nexuschat
sudo bash deploy/install.sh chat.example.com
```

Скрипт ставит Docker, создаёт случайные пароли в `deploy/.env` (сохраните копию),
при нехватке памяти добавляет подкачку и поднимает `deploy/docker-compose.yml`:
NexusChat, PostgreSQL, MinIO и Caddy, который сам получает и продлевает HTTPS-сертификат.
Наружу открыты только порты 80 и 443. Обновить сервер: `git pull && sudo bash deploy/install.sh`.

## Приложение для Windows

`desktop/` — тонкая оболочка Tauri: окно открывает веб-клиент с вашего сервера
(сервер отдаёт его сам, если задан `CLIENT_DIST`). Поэтому:

- **интерфейс** обновляется вместе с сервером, без переустановки и без вопросов;
- **само приложение** (оболочку) обновляет встроенный модуль Tauri: при запуске и раз в 6 часов
  оно проверяет GitHub Releases и спрашивает «Доступна новая версия… Обновить сейчас?».
  После «Обновить» ставит новую версию и перезапускается; «Позже» не спрашивает снова до перезапуска.
  Обновление подписано вашим ключом: чужой файл не установится.
- Установка в профиль пользователя, без прав администратора, на русском.
- Странице с сервера не выдано ни одного разрешения Tauri: доступ к ПК у неё как у сайта в браузере.

### Один раз настроить

1. Разверните сервер по HTTPS с `CLIENT_DIST` (собранный `client/dist`), например `https://chat.example.com`.
2. Создайте ключ подписи обновлений и **сохраните его в надёжном месте** — без него нельзя
   выпустить обновление для уже установленных копий:
   ```bash
   cd desktop && pnpm install && pnpm tauri signer generate -w ~/.nexuschat/updater.key
   ```
3. В репозитории на GitHub: Settings → Secrets and variables → Actions.
   - Secrets: `TAURI_SIGNING_PRIVATE_KEY` (содержимое `updater.key`), `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
   - Variables: `TAURI_UPDATER_PUBKEY` (содержимое `updater.key.pub`), `NEXUS_SERVER_URL` (адрес из п. 1).

### Выпустить версию

```bash
git tag desktop-v0.1.0 && git push origin desktop-v0.1.0
```

GitHub Actions (`.github/workflows/desktop-release.yml`) соберёт `NexusChat_0.1.0_x64-setup.exe`,
опубликует релиз и `latest.json` для обновлений. Следующий тег (`desktop-v0.1.1`) — и установленные
копии предложат обновиться. Версия берётся из тега; адрес сервера, ключ и адрес обновлений
подставляются при сборке поверх `desktop/src-tauri/tauri.conf.json`.

Локально: `cd desktop && pnpm tauri dev` открывает оболочку с `http://localhost:5173`.

## Проверки

```bash
cd server && cargo clippy --all-targets && cargo test   # интеграционным тестам нужны Postgres и S3 из .env
cd client && pnpm build
```

## Миграции

Лежат в `server/migrations/<версия>_<имя>/{up,down}.sql`, встраиваются в бинарник
и применяются при старте сервера. Схема для запросов — `server/src/schema.rs`.
