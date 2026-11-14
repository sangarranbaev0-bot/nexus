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
    /// Непрочитанные чужие сообщения, максимум 100 (клиент покажет «99+»).
    pub unread_count: i32,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ChannelKind {
    #[default]
    Text,
    Voice,
}

impl ChannelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChannelKind::Text => "text",
            ChannelKind::Voice => "voice",
        }
    }

    pub fn parse(s: &str) -> Self {
        if s == "voice" {
            ChannelKind::Voice
        } else {
            ChannelKind::Text
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
#[ts(export)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(default)]
    #[ts(optional)]
    pub kind: Option<ChannelKind>,
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
