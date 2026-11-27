//! Контракт клиент ↔ сервер. TS-типы для клиента генерируются отсюда (`cargo test`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HealthResponse {
    pub db: bool,
    #[ts(type = "number")]
    pub server_time: i64,
}

/// Тело любой ошибки API.
#[derive(Serialize, Deserialize, TS, Debug)]
#[ts(export)]
pub struct ApiError {
    pub error: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct User {
    #[ts(type = "number")]
    pub id: i64,
    pub email: String,
    pub username: String,
    pub about: String,
    pub banner_url: String,
    pub pronouns: String,
    #[ts(type = "number | null")]
    pub accent_color: Option<i32>,
    pub status_text: String,
    pub status_emoji: String,
    /// Unix-время в мс; `null` — бессрочно.
    #[ts(type = "number | null")]
    pub status_expires_at: Option<i64>,
    #[ts(type = "string")]
    pub activity_name: String,
    #[ts(type = "string")]
    pub activity_type: String,
    #[ts(type = "string")]
    pub notify_mode: String,
    pub dnd: bool,
    pub two_factor_enabled: bool,
    #[serde(default)]
    pub badge: String,
}

/// Частичное обновление профиля.
#[derive(Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    #[ts(optional)]
    pub about: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub banner_url: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub pronouns: Option<String>,
    #[serde(default)]
    #[ts(optional, type = "number | null")]
    pub accent_color: Option<Option<i32>>,
    #[serde(default)]
    #[ts(optional)]
    pub status_text: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub status_emoji: Option<String>,
    #[serde(default)]
    #[ts(optional, type = "number | null")]
    pub status_expires_at: Option<Option<i64>>,
    #[serde(default)]
    pub activity_name: Option<String>,
    #[serde(default)]
    pub activity_type: Option<String>,
    #[serde(default)]
    pub notify_mode: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub dnd: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub badge: Option<String>,
}

/// Тикет поддержки на сервере.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Ticket {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    #[ts(type = "number")]
    pub creator_id: i64,
    pub status: String,
    pub title: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub closed_at: Option<i64>,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Note {
    #[ts(type = "number")]
    pub target_id: i64,
    pub note: String,
    pub nickname: String,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateNoteRequest {
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub nickname: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Session {
    #[ts(type = "number")]
    pub id: i64,
    pub device_label: String,
    pub user_agent: String,
    pub ip: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub expires_at: i64,
    #[ts(type = "number | null")]
    pub last_used_at: Option<i64>,
    pub revoked: bool,
    pub current: bool,
}

#[derive(Serialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwoFactorSetup {
    pub secret: String,
    pub uri: String,
    pub recovery_codes: Vec<String>,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwoFactorCode {
    pub code: String,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoginWithCode {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub recovery_code: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportResponse {
    pub data: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Одна предыдущая версия сообщения.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageEdit {
    #[ts(type = "number")]
    pub id: i64,
    pub content: String,
    #[ts(type = "number")]
    pub edited_at: i64,
}

/// Ответ на register/login/refresh. Refresh-токен приходит отдельно, в httpOnly cookie.
#[derive(Serialize, Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuthResponse {
    pub access_token: String,
    pub user: User,
}

/// Сообщения от клиента по WebSocket. Первым обязано идти `auth`.
#[derive(Deserialize, Serialize, TS, Debug, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ClientMessage {
    Auth {
        token: String,
    },
    Ping {
        #[ts(type = "number")]
        sent_at: i64,
    },
    /// Пользователь печатает в канале. Клиент шлёт не чаще раза в несколько секунд.
    Typing {
        #[ts(type = "number")]
        channel_id: i64,
    },
    /// Бездействие вкладки: `true` после 5 минут без активности, `false` при возврате.
    SetIdle {
        idle: bool,
    },
    /// Войти в голосовой канал или звонок в ЛС (из другого голосового выйдет сам).
    VoiceJoin {
        #[ts(type = "number")]
        channel_id: i64,
    },
    VoiceLeave,
    VoiceSetFlags {
        flags: VoiceFlags,
    },
    /// WebRTC-сигнал (SDP/ICE) участнику того же голосового канала. Содержимое сервер не читает.
    VoiceSignal {
        #[ts(type = "number")]
        to_user_id: i64,
        #[ts(type = "unknown")]
        data: serde_json::Value,
    },
}

/// Сообщения от сервера по WebSocket.
#[derive(Deserialize, Serialize, TS, Debug, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ServerMessage {
    Hello {
        /// Глобальный id подключения (строка присутствия), уникален между экземплярами.
        #[ts(type = "number")]
        conn_id: i64,
        #[ts(type = "number")]
        user_id: i64,
    },
    Pong {
        #[ts(type = "number")]
        sent_at: i64,
        #[ts(type = "number")]
        server_time: i64,
    },
    Error {
        message: String,
    },
    MessageCreated {
        message: Message,
    },
    MessageUpdated {
        message: Message,
    },
    MessageDeleted {
        #[ts(type = "number")]
        channel_id: i64,
        #[ts(type = "number")]
        message_id: i64,
    },
    Typing {
        #[ts(type = "number")]
        channel_id: i64,
        #[ts(type = "number")]
        user_id: i64,
        username: String,
    },
    /// Каналы сервера изменились: клиент перезагружает их список.
    ChannelsChanged {
        #[ts(type = "number")]
        server_id: i64,
    },
    /// Роли сервера или назначения изменились: клиент перезагружает роли и участников.
    RolesChanged {
        #[ts(type = "number")]
        server_id: i64,
    },
    /// Набор своих эмодзи сервера изменился: клиент перечитывает их.
    EmojisChanged {
        #[ts(type = "number")]
        server_id: i64,
    },
    /// Вошёл в голос или изменил флаги (микрофон, камера, экран).
    VoiceState {
        participant: VoiceParticipant,
    },
    VoiceLeft {
        #[ts(type = "number")]
        channel_id: i64,
        #[ts(type = "number")]
        user_id: i64,
    },
    VoiceSignal {
        #[ts(type = "number")]
        from_user_id: i64,
        #[ts(type = "unknown")]
        data: serde_json::Value,
    },
    PresenceChanged {
        #[ts(type = "number")]
        user_id: i64,
        status: Presence,
    },
    /// Канал прочитан в другой вкладке или на другом устройстве этого пользователя.
    ChannelRead {
        /// `null` — личные сообщения.
        #[ts(type = "number | null")]
        server_id: Option<i64>,
        #[ts(type = "number")]
        channel_id: i64,
    },
    /// Список друзей, заявок или блокировок изменился: клиент перечитывает их.
    FriendsChanged,
    /// Состав или название личного диалога/группы изменились.
    DmsChanged,
    /// Новое уведомление пришло этому пользователю (только его подключениям).
    NotificationCreated {
        notification: Notification,
    },
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Message {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub channel_id: i64,
    #[ts(type = "number")]
    pub author_id: i64,
    pub author_username: String,
    #[serde(default)]
    pub author_badge: String,
    pub content: String,
    /// Unix-время в мс.
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub edited_at: Option<i64>,
    pub attachments: Vec<Attachment>,
    pub reactions: Vec<Reaction>,
    pub reply_to: Option<ReplyPreview>,
    pub pinned: bool,
    /// Тип системного события (`memberJoin`, `channelCreate`, `roleUpdate`, `welcome`); `null` — обычное.
    pub system_kind: Option<String>,
    /// Дополнительные данные события (имя канала/роль).
    pub system_detail: Option<String>,
    /// Блок «Переслано от»; `null` — сообщение не пересланное.
    pub forwarded_from: Option<ForwardPreview>,
    /// Корень ветки в форум-канале; `null` — обычное сообщение.
    #[ts(type = "number | null")]
    pub thread_root_id: Option<i64>,
    pub thread_archived: bool,
    pub thread_locked: bool,
    #[ts(type = "boolean")]
    pub has_embed: bool,
}

/// Свой эмодзи сервера: рисуется по `:name:` в сообщениях.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Emoji {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    pub name: String,
    /// Подписанная ссылка на PNG; действует 1–2 суток.
    pub url: String,
}

/// Снимок исходного сообщения в пересланной копии.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForwardPreview {
    #[ts(type = "number | null")]
    pub message_id: Option<i64>,
    #[ts(type = "number | null")]
    pub author_id: Option<i64>,
    pub author_username: String,
    pub content: String,
    #[ts(type = "number | null")]
    pub channel_id: Option<i64>,
}

/// Превью ссылки (OpenGraph): извлекается бэком при отправке сообщения.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmbedPreview {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub site_name: Option<String>,
}

/// Стикерпак (коллекция стикеров сервера).
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StickerPack {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    pub name: String,
    pub description: String,
    #[ts(type = "number")]
    pub created_by: i64,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Стикер внутри пака.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sticker {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub pack_id: i64,
    pub name: String,
    pub emoji: String,
    pub content_type: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    pub storage_key: String,
    pub tags: Vec<String>,
    #[ts(type = "number")]
    pub position: i64,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Запрос на создание стикерпака (admin-only).
#[derive(Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateStickerPackRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Запрос на добавление стикера в пак (admin-only).
#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateStickerRequest {
    pub name: String,
    pub emoji: String,
    pub content_type: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    pub storage_key: String,
    pub tags: Vec<String>,
    #[ts(type = "number")]
    pub position: i32,
}

/// Краткая цитата сообщения, на которое отвечают.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReplyPreview {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub author_id: i64,
    pub author_username: String,
    /// Обрезанный текст (200 символов) или пустая строка, если было только вложение.
    pub content: String,
}

/// Реакция на сообщение: эмодзи и все, кто его поставил (по порядку добавления).
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Reaction {
    pub emoji: String,
    #[ts(type = "Array<number>")]
    pub user_ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Attachment {
    #[ts(type = "number")]
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    /// Подписанная ссылка на скачивание; действует 1–2 суток.
    pub url: String,
}

/// Тело POST и PATCH сообщения. Вложения задаются только при создании.
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageContent {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    #[ts(optional, type = "Array<number>")]
    pub attachment_ids: Option<Vec<i64>>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub reply_to_id: Option<i64>,
}

/// Тело POST `/api/messages/{id}/forward`: канал, куда переслать.
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForwardRequest {
    #[ts(type = "number")]
    pub channel_id: i64,
}

/// Параметры поиска сообщений.
#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchParams {
    /// Текстовый запрос (поддерживает фильтры from:, has:, before:, after:).
    pub q: String,
    /// Максимальное количество результатов (1-100).
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub limit: Option<i64>,
    /// Искать до этого ID сообщения (exclusive).
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub before: Option<i64>,
    /// Искать после этого ID сообщения (exclusive).
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub after: Option<i64>,
}

/// Результат поиска: сообщение + контекст (канал/сервер).
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchResult {
    #[ts(type = "number")]
    pub message_id: i64,
    #[ts(type = "number")]
    pub channel_id: i64,
    #[ts(type = "number")]
    pub server_id: Option<i64>,
    pub channel_name: String,
    pub message: Message,
}

/// Вопрос онбординга: за правильный ответ выдаётся роль.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OnboardingQuestion {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    pub question: String,
    #[ts(type = "number")]
    pub role_id: i64,
    pub position: i32,
    pub answered: bool,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateOnboardingQuestionRequest {
    pub question: String,
    #[ts(type = "number")]
    pub role_id: i64,
}

/// Событие сервера.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerEvent {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    pub name: String,
    pub description: String,
    /// Unix-время в мс.
    #[ts(type = "number")]
    pub starts_at: i64,
    #[ts(type = "number | null")]
    pub channel_id: Option<i64>,
    #[ts(type = "number")]
    pub creator_id: i64,
    pub interested_count: i32,
    pub interested: bool,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateServerEventRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Unix-время в мс.
    #[ts(type = "number")]
    pub starts_at: i64,
    #[serde(default)]
    #[ts(optional, type = "number | null")]
    pub channel_id: Option<Option<i64>>,
    /// Отправить напоминание участникам, отметившим «Интересно», за час до начала.
    #[serde(default)]
    pub reminder: bool,
}

/// Тег форум-ветки.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThreadTag {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub channel_id: i64,
    pub name: String,
    #[ts(type = "number | null")]
    pub color: Option<i32>,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateThreadTagRequest {
    pub name: String,
    #[serde(default)]
    #[ts(optional, type = "number | null")]
    pub color: Option<Option<i32>>,
}

/// Шаблон сервера для создания нового.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerTemplate {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub rules: String,
    #[serde(default)]
    pub banner_url: String,
    #[serde(default)]
    pub channels: Vec<TemplateChannel>,
    #[serde(default)]
    pub roles: Vec<TemplateRole>,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemplateChannel {
    pub name: String,
    pub kind: ChannelKind,
    pub is_forum: bool,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemplateRole {
    pub name: String,
    #[ts(type = "number | null")]
    pub color: Option<i32>,
    #[ts(type = "number")]
    pub permissions: i64,
    pub position: i32,
    pub hoist: bool,
    pub mentionable: bool,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateFromTemplateRequest {
    pub template: ServerTemplate,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PruneResponse {
    pub removed: i32,
}

/// Параметры поиска участников сервера.
#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MemberSearchParams {
    pub q: String,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub limit: Option<i64>,
}

/// Ответ поиска участников.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MemberSearchResponse {
    pub results: Vec<Member>,
    pub has_more: bool,
}

/// Ответ поиска: список результатов + есть ли ещё.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub has_more: bool,
}

/// Параметры глобального поиска по серверу.
#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerSearchParams {
    /// Текстовый запрос (поддерживает фильтры from:, has:, has:link).
    pub q: String,
    /// Искать только в этом канале (null = все каналы сервера).
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub channel_id: Option<i64>,
    /// Максимальное количество результатов (1-100).
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub limit: Option<i64>,
}

/// Результат глобального поиска по серверу.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerSearchResult {
    #[ts(type = "number")]
    pub message_id: i64,
    #[ts(type = "number")]
    pub channel_id: i64,
    pub channel_name: String,
    pub message: Message,
}

/// Ответ глобального поиска.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerSearchResponse {
    pub results: Vec<ServerSearchResult>,
    pub has_more: bool,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Роль участника сервера. Порядок вариантов задаёт старшинство: Member < Admin < Owner.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Role {
    Member,
    Admin,
    Owner,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Member => "member",
            Role::Admin => "admin",
            Role::Owner => "owner",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "member" => Some(Role::Member),
            "admin" => Some(Role::Admin),
            "owner" => Some(Role::Owner),
            _ => None,
        }
    }
}

/// Сервер (гильдия) с ролью текущего пользователя в нём.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Server {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    #[ts(type = "number")]
    pub owner_id: i64,
    pub my_role: Role,
    /// Итоговые права участника на сервере (битовая маска, как в `permissions.rs`).
    #[ts(type = "number")]
    pub my_permissions: i64,
    /// Сумма непрочитанных по каналам (больше ста почти не бывает, считаем максимум 100).
    pub unread_count: i32,
    /// Канал событий сервера (вход, создание канала, роли); `null` — не задан.
    #[ts(type = "number | null")]
    pub system_channel_id: Option<i64>,
    /// Голосовой AFK-канал; `null` — не задан.
    #[ts(type = "number | null")]
    pub afk_channel_id: Option<i64>,
    /// Описание сервера (Markdown).
    pub description: String,
    /// Правила сервера (Markdown).
    pub rules: String,
    /// URL баннера сервера; пустая строка — нет.
    pub banner_url: String,
    /// Канал объявлений; `null` — не задан.
    #[ts(type = "number | null")]
    pub announcement_channel_id: Option<i64>,
    pub boost_count: i32,
    pub boost_level: i32,
}

/// Бустер сервера (кто и когда бустнул).
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoostEntry {
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    #[ts(type = "number")]
    pub boosted_at: i64,
}

/// Информация о системе бустов сервера.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerBoostInfo {
    pub level: i32,
    pub count: i32,
    pub boosters: Vec<BoostEntry>,
}

/// Частичное обновление настроек сервера (только owner/admin).
#[derive(Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateServerRequest {
    #[serde(default)]
    #[ts(optional, type = "number | null")]
    pub system_channel_id: Option<Option<i64>>,
    #[serde(default)]
    #[ts(optional, type = "number | null")]
    pub afk_channel_id: Option<Option<i64>>,
    #[serde(default)]
    #[ts(optional, type = "number | null")]
    pub announcement_channel_id: Option<Option<i64>>,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub rules: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub banner_url: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub name: Option<String>,
}

/// Роль, выдаваемая за реакцию на сообщение.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReactionRole {
    #[ts(type = "number")]
    pub message_id: i64,
    pub emoji: String,
    #[ts(type = "number")]
    pub role_id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateReactionRoleRequest {
    pub emoji: String,
    #[ts(type = "number")]
    pub role_id: i64,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Channel {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    pub name: String,
    pub position: i32,
    pub kind: ChannelKind,
    /// Категория канала; `null` — без категории.
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    /// Форум-канал: сообщения являются ветками.
    pub is_forum: bool,
    /// Непрочитанные чужие сообщения, максимум 100 (клиент пишет «99+»).
    pub unread_count: i32,
    /// Описание канала (тема).
    pub topic: String,
    ///Slowmode в секундах; 0 — без ограничений.
    pub slowmode_seconds: i32,
    /// Флаг NSFW-канала.
    pub nsfw: bool,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ChannelKind {
    #[default]
    Text,
    Voice,
    Stage,
}

impl ChannelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChannelKind::Text => "text",
            ChannelKind::Voice => "voice",
            ChannelKind::Stage => "stage",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "voice" => ChannelKind::Voice,
            "stage" => ChannelKind::Stage,
            _ => ChannelKind::Text,
        }
    }
}

/// Участник голосового канала или звонка в ЛС.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VoiceParticipant {
    #[ts(type = "number")]
    pub channel_id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    /// Подключение (вкладка), из которого пользователь в голосе; см. `Hello.connId`.
    #[ts(type = "number")]
    pub connection_id: i64,
    #[serde(flatten)]
    pub flags: VoiceFlags,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Default)]
#[ts(export)]
pub struct VoiceFlags {
    pub muted: bool,
    pub deafened: bool,
    pub camera: bool,
    pub screen: bool,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Member {
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    pub role: Role,
    /// Назначенные пользовательские роли (без `@everyone`).
    #[ts(type = "Array<number>")]
    pub role_ids: Vec<i64>,
    pub status: Presence,
    pub nickname: String,
    pub avatar_url: String,
    /// До время отключения связи (NULL — активен). Unix-время в мс.
    #[ts(type = "number | null")]
    pub communication_disabled_until: Option<i64>,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateMemberProfileRequest {
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub avatar_url: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Presence {
    Online,
    Idle,
    Offline,
}

impl Presence {
    pub fn parse(s: &str) -> Self {
        match s {
            "online" => Presence::Online,
            "idle" => Presence::Idle,
            _ => Presence::Offline,
        }
    }
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MarkReadRequest {
    #[ts(type = "number")]
    pub message_id: i64,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Invite {
    pub code: String,
    #[ts(type = "number")]
    pub server_id: i64,
    /// Unix-время в мс; `null` — бессрочный.
    #[ts(type = "number | null")]
    pub expires_at: Option<i64>,
    pub max_uses: Option<i32>,
    pub uses: i32,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateServerRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(default)]
    #[ts(optional)]
    pub kind: Option<ChannelKind>,
    #[serde(default)]
    #[ts(optional)]
    pub is_forum: Option<bool>,
}

#[derive(Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateInviteRequest {
    #[ts(optional)]
    pub expires_in_hours: Option<u32>,
    #[ts(optional)]
    pub max_uses: Option<i32>,
}

/// Owner меняет роль участника: только `admin` или `member`.
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateMemberRequest {
    pub role: Role,
}

/// Запись о бане (кирке/таймауте).
#[derive(Serialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BanInfo {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    #[ts(type = "number")]
    pub actor_id: i64,
    pub actor_username: String,
    pub reason: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Тело POST/PATCH `/api/servers/{id}/members/{user_id}/ban`.
#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BanRequest {
    #[serde(default)]
    pub reason: String,
}

/// Пользовательская роль сервера. `position` больше — старше.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleInfo {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    pub name: String,
    /// Цвет в 0xRRGGBB; `null` — без цвета.
    #[ts(type = "number | null")]
    pub color: Option<i32>,
    /// Битовая маска прав; биты описаны в серверном `permissions.rs`.
    #[ts(type = "number")]
    pub permissions: i64,
    pub position: i32,
    pub hoist: bool,
    pub mentionable: bool,
    /// `@everyone`: существует на каждом сервере, удалять и переименовывать нельзя.
    pub is_default: bool,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateRoleRequest {
    pub name: String,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    #[ts(type = "number")]
    pub permissions: Option<i64>,
    #[serde(default)]
    pub hoist: Option<bool>,
    #[serde(default)]
    pub mentionable: Option<bool>,
}

/// Частичное обновление роли. `color: -1` очищает цвет.
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateRoleRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    #[ts(type = "number")]
    pub permissions: Option<i64>,
    #[serde(default)]
    pub hoist: Option<bool>,
    #[serde(default)]
    pub mentionable: Option<bool>,
    #[serde(default)]
    pub position: Option<i32>,
}

/// Полная замена пользовательских ролей участника (без `@everyone`).
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AssignRolesRequest {
    #[ts(type = "Array<number>")]
    pub role_ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Category {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    pub name: String,
    pub position: i32,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateCategoryRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateCategoryRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub position: Option<i32>,
}

/// Частичное обновление канала. `categoryId: 0` убирает канал из категории.
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateChannelRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[ts(type = "number")]
    pub category_id: Option<i64>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    #[ts(optional)]
    pub is_forum: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub topic: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub slowmode_seconds: Option<i32>,
    #[serde(default)]
    #[ts(optional)]
    pub nsfw: Option<bool>,
}

/// Переопределение прав канала для роли или участника.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Overwrite {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub channel_id: i64,
    #[ts(type = "number | null")]
    pub role_id: Option<i64>,
    #[ts(type = "number | null")]
    pub user_id: Option<i64>,
    #[ts(type = "number")]
    pub allow: i64,
    #[ts(type = "number")]
    pub deny: i64,
}

/// Создать или заменить переопределение. Ровно одно из `roleId`/`userId`.
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpsertOverwriteRequest {
    #[serde(default)]
    #[ts(optional)]
    #[ts(type = "number")]
    pub role_id: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    #[ts(type = "number")]
    pub user_id: Option<i64>,
    #[ts(type = "number")]
    pub allow: i64,
    #[ts(type = "number")]
    pub deny: i64,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditEntry {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub server_id: i64,
    #[ts(type = "number")]
    pub actor_id: i64,
    pub actor_username: String,
    pub action: String,
    #[ts(type = "string | null")]
    pub target: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Диалог в личных сообщениях. `id` — id канала: лента и отметки прочтения общие с серверными.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DmChannel {
    #[ts(type = "number")]
    pub id: i64,
    /// Собеседник (в группе — первый по алфавиту другой участник).
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    pub status: Presence,
    pub unread_count: i32,
    pub is_group: bool,
    /// Название группы; пустое — показываем имена участников.
    pub name: String,
    /// Все участники, кроме вас.
    pub members: Vec<DmMember>,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DmMember {
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
}

#[derive(Serialize, Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateGroupRequest {
    pub usernames: Vec<String>,
    #[ts(optional)]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, TS, Debug)]
#[ts(export)]
pub struct NameRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpenDmRequest {
    pub username: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Friend {
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    pub status: Presence,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FriendRequest {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub from_user_id: i64,
    pub from_username: String,
    #[ts(type = "number")]
    pub to_user_id: i64,
    pub to_username: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FriendRequests {
    pub incoming: Vec<FriendRequest>,
    pub outgoing: Vec<FriendRequest>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SendFriendRequest {
    pub username: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlockedUser {
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TimeoutRequest {
    #[ts(type = "number")]
    pub seconds: i64,
}

/// Порог уведомлений: все сообщения, только @упоминания или ничего.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum NotifyLevel {
    All,
    Mentions,
    None,
}

impl NotifyLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            NotifyLevel::All => "all",
            NotifyLevel::Mentions => "mentions",
            NotifyLevel::None => "none",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(NotifyLevel::All),
            "mentions" => Some(NotifyLevel::Mentions),
            "none" => Some(NotifyLevel::None),
            _ => None,
        }
    }
}

/// Настройка уведомлений для сервера или канала (ровно одно из двух полей задано).
/// `mutedUntil` — Unix-время в мс, до которого уведомления подавлены полностью.
/// `suppressEveryone` глушит @everyone/@here, даже если у автора есть право.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotifySetting {
    #[ts(type = "number | null")]
    pub server_id: Option<i64>,
    #[ts(type = "number | null")]
    pub channel_id: Option<i64>,
    pub level: NotifyLevel,
    #[ts(type = "number | null")]
    pub muted_until: Option<i64>,
    pub suppress_everyone: bool,
}

/// Почему пришло уведомление.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum NotificationKind {
    Mention,
    Reply,
    Word,
    Everyone,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NotificationKind::Mention => "mention",
            NotificationKind::Reply => "reply",
            NotificationKind::Word => "word",
            NotificationKind::Everyone => "everyone",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mention" => Some(NotificationKind::Mention),
            "reply" => Some(NotificationKind::Reply),
            "word" => Some(NotificationKind::Word),
            "everyone" => Some(NotificationKind::Everyone),
            _ => None,
        }
    }
}

/// Запись центра уведомлений.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Notification {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: NotificationKind,
    #[ts(type = "number | null")]
    pub server_id: Option<i64>,
    #[ts(type = "number")]
    pub channel_id: i64,
    #[ts(type = "number")]
    pub message_id: i64,
    #[ts(type = "number")]
    pub actor_id: i64,
    pub actor_username: String,
    /// Обрезанный текст сообщения (200 символов).
    pub preview: String,
    /// Unix-время в мс.
    #[ts(type = "number")]
    pub created_at: i64,
    pub read: bool,
    /// Название сервера или собеседника для подписи.
    pub location: String,
}

/// Слово-триггер пользователя.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotifyWord {
    pub word: String,
    pub case_sensitive: bool,
}

/// Тело добавления слова-триггера.
#[derive(Serialize, Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddNotifyWord {
    pub word: String,
    pub case_sensitive: bool,
}

/// Статистика сервера для дашборда.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerStats {
    #[ts(type = "number")]
    pub member_count: i64,
    #[ts(type = "number")]
    pub channel_count: i64,
    #[ts(type = "number")]
    pub message_count_24h: i64,
    #[ts(type = "number")]
    pub online_count: i64,
}

/// Автоправило модерации.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AutoModRule {
    #[ts(type = "number")]
    pub id: i64,
    pub pattern: String,
    pub action: String,
    pub note: String,
}

#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateAutoModRuleRequest {
    pub pattern: String,
    pub action: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Вебхук канала.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Webhook {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub avatar_url: String,
    #[ts(type = "number")]
    pub channel_id: i64,
    /// Токен только при создании; в списке — красится на `***`.
    pub token: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Запрос создания вебхука.
#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateWebhookRequest {
    pub name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// OAuth-приложение (упрощённая регистрация).
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OAuthApp {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub owner_id: i64,
    pub name: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub redirect_uris: Vec<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Запрос регистрации OAuth-приложения.
#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateOAuthAppRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

/// Ответ на создание OAuth-приложения (однократно возвращается secret).
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateOAuthAppResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub owner_id: i64,
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub redirect_uris: Vec<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Черновик сообщения.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Draft {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    #[ts(type = "number")]
    pub channel_id: i64,
    pub content: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

/// Запрос сохранения черновика.
#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveDraftRequest {
    pub content: String,
}

/// Состояние запланированного сообщения.
#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduledMessage {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub author_id: i64,
    #[ts(type = "number")]
    pub channel_id: i64,
    pub content: String,
    #[ts(type = "number")]
    pub scheduled_at: i64,
    pub status: String,
    #[ts(type = "number")]
    pub sent_at: Option<i64>,
    pub error_msg: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Запрос создания запланированного сообщения.
#[derive(Deserialize, TS, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateScheduledMessageRequest {
    #[ts(type = "number")]
    pub channel_id: i64,
    pub content: String,
    /// Unix timestamp (ms) когда отправить.
    #[ts(type = "number")]
    pub scheduled_for: i64,
}
