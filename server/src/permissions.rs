//! Битовая модель прав, базовые права ролей и переопределения на канал.
//!
//! Старые роли `owner/admin/member` остаются «уровнем» участника; поверх них
//! назначаются пользовательские роли. Права = базовые права уровня ∪ права ролей,
//! затем права канала (deny, потом allow). Владелец имеет всё и обходит проверки.

use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    error::{AppError, AppResult},
    protocol::Role,
    schema::{channel_overwrites, channels, member_roles, roles},
    servers::role_in,
};

pub const VIEW_CHANNEL: i64 = 1 << 0;
pub const SEND_MESSAGES: i64 = 1 << 1;
pub const ATTACH_FILES: i64 = 1 << 2;
pub const ADD_REACTIONS: i64 = 1 << 3;
pub const MENTION_EVERYONE: i64 = 1 << 4;
pub const READ_MESSAGE_HISTORY: i64 = 1 << 5;
pub const MANAGE_MESSAGES: i64 = 1 << 6;
pub const MANAGE_CHANNELS: i64 = 1 << 7;
pub const MANAGE_ROLES: i64 = 1 << 8;
pub const KICK_MEMBERS: i64 = 1 << 9;
pub const BAN_MEMBERS: i64 = 1 << 10;
pub const TIMEOUT_MEMBERS: i64 = 1 << 11;
pub const CREATE_INVITE: i64 = 1 << 12;
pub const CHANGE_NICKNAME: i64 = 1 << 13;
pub const MANAGE_NICKNAMES: i64 = 1 << 14;
pub const CREATE_THREADS: i64 = 1 << 15;
pub const MANAGE_THREADS: i64 = 1 << 16;
pub const VIEW_AUDIT_LOG: i64 = 1 << 17;
pub const CONNECT_VOICE: i64 = 1 << 18;
pub const SPEAK: i64 = 1 << 19;
pub const MUTE_MEMBERS: i64 = 1 << 20;
pub const DEAFEN_MEMBERS: i64 = 1 << 21;
pub const MOVE_MEMBERS: i64 = 1 << 22;
pub const PRIORITY_SPEAKER: i64 = 1 << 23;
pub const STREAM: i64 = 1 << 24;
pub const ADMINISTRATOR: i64 = 1 << 25;

/// Все описанные биты.
pub const ALL: i64 = (1 << 26) - 1;

/// Права обычного участника сервера.
pub const MEMBER_BASE: i64 = VIEW_CHANNEL
    | SEND_MESSAGES
    | ATTACH_FILES
    | ADD_REACTIONS
    | READ_MESSAGE_HISTORY
    | CREATE_INVITE
    | CHANGE_NICKNAME
    | CONNECT_VOICE
    | SPEAK
    | STREAM;

/// Права администратора (всё, кроме `ADMINISTRATOR`: он остаётся только у владельца
/// и у ролей, которым его выдали явно).
pub const ADMIN_BASE: i64 = ALL & !ADMINISTRATOR;

pub fn base_for(role: Role) -> i64 {
    match role {
        Role::Owner => ALL,
        Role::Admin => ADMIN_BASE,
        Role::Member => MEMBER_BASE,
    }
}

pub fn has(bits: i64, permission: i64) -> bool {
    bits & permission == permission
}

pub struct MemberPerms {
    pub role: Role,
    pub user_id: i64,
    /// Итоговые права с учётом пользовательских ролей (без переопределений канала).
    pub bits: i64,
    /// Пользовательские роли + `@everyone`; используются для переопределений.
    pub role_ids: Vec<i64>,
}

pub async fn member_perms(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    user_id: i64,
) -> AppResult<MemberPerms> {
    let role = role_in(conn, server_id, user_id).await?;
    let mut bits = base_for(role);

    let assigned: Vec<(i64, i64)> = member_roles::table
        .inner_join(roles::table)
        .filter(member_roles::server_id.eq(server_id))
        .filter(member_roles::user_id.eq(user_id))
        .select((member_roles::role_id, roles::permissions))
        .load(conn)
        .await?;
    let mut role_ids = Vec::with_capacity(assigned.len() + 1);
    for (role_id, permissions) in assigned {
        role_ids.push(role_id);
        bits |= permissions;
    }

    let default_id: Option<i64> = roles::table
        .filter(roles::server_id.eq(server_id))
        .filter(roles::is_default.eq(true))
        .select(roles::id)
        .first(conn)
        .await
        .optional()?;
    if let Some(id) = default_id {
        role_ids.push(id);
    }

    if has(bits, ADMINISTRATOR) {
        bits = ALL;
    }
    Ok(MemberPerms {
        role,
        user_id,
        bits,
        role_ids,
    })
}

/// Требует право на сервере. Постороннему отвечает 404 (через `role_in`), участнику без права — 403.
pub async fn require_permission(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    user_id: i64,
    permission: i64,
) -> AppResult<MemberPerms> {
    let perms = member_perms(conn, server_id, user_id).await?;
    if has(perms.bits, permission) {
        Ok(perms)
    } else {
        Err(AppError::forbidden())
    }
}

/// Право на конкретном канале с учётом переопределений.
pub async fn require_channel_permission(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    user_id: i64,
    channel_id: i64,
    permission: i64,
) -> AppResult<MemberPerms> {
    require_channel_permissions(conn, server_id, user_id, channel_id, permission).await
}

/// Как `require_channel_permission`, но проверяет сразу набор битов (все должны быть).
pub async fn require_channel_permissions(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    user_id: i64,
    channel_id: i64,
    permissions: i64,
) -> AppResult<MemberPerms> {
    let perms = member_perms(conn, server_id, user_id).await?;
    let bits = channel_bits(conn, &perms, channel_id).await?;
    if has(bits, permissions) {
        Ok(perms)
    } else {
        Err(AppError::forbidden())
    }
}

type OverwriteTuple = (i64, Option<i64>, Option<i64>, i64, i64);
type ChannelOverwrites = HashMap<i64, Vec<(Option<i64>, Option<i64>, i64, i64)>>;

/// Каналы сервера, которые участник видит с учётом переопределений.
/// `None` - видит все (владелец или `ADMINISTRATOR`).
pub async fn visible_channel_ids(
    conn: &mut AsyncPgConnection,
    perms: &MemberPerms,
    server_id: i64,
) -> AppResult<Option<HashSet<i64>>> {
    if perms.role == Role::Owner || has(perms.bits, ADMINISTRATOR) {
        return Ok(None);
    }

    let overwrites: Vec<OverwriteTuple> = channel_overwrites::table
        .inner_join(channels::table.on(channels::id.eq(channel_overwrites::channel_id)))
        .filter(channels::server_id.eq(server_id))
        .select((
            channel_overwrites::channel_id,
            channel_overwrites::role_id,
            channel_overwrites::user_id,
            channel_overwrites::allow,
            channel_overwrites::deny,
        ))
        .load(conn)
        .await?;
    let mut by_channel: ChannelOverwrites = HashMap::new();
    for (channel_id, role_id, user_id, allow, deny) in overwrites {
        by_channel
            .entry(channel_id)
            .or_default()
            .push((role_id, user_id, allow, deny));
    }

    let ids: Vec<i64> = channels::table
        .filter(channels::server_id.eq(server_id))
        .select(channels::id)
        .load(conn)
        .await?;

    let mut visible = HashSet::with_capacity(ids.len());
    for channel_id in ids {
        let mut bits = perms.bits;
        let mut personal = None;
        for (role_id, user_id, allow, deny) in by_channel.get(&channel_id).into_iter().flatten() {
            if let Some(target) = user_id {
                if *target == perms.user_id {
                    personal = Some((*allow, *deny));
                }
            } else if role_id.is_some_and(|id| perms.role_ids.contains(&id)) {
                bits = (bits & !deny) | allow;
            }
        }
        if let Some((allow, deny)) = personal {
            bits = (bits & !deny) | allow;
        }
        if has(bits, VIEW_CHANNEL) {
            visible.insert(channel_id);
        }
    }
    Ok(Some(visible))
}

#[derive(Queryable)]
struct OverwriteRow {
    role_id: Option<i64>,
    user_id: Option<i64>,
    allow: i64,
    deny: i64,
}

/// Права на конкретный канал с учётом переопределений ролей и участника.
/// Порядок как в Discord: стартуем с базовых, применяем deny→allow для ролей,
/// затем личное переопределение участника.
pub async fn channel_bits(
    conn: &mut AsyncPgConnection,
    perms: &MemberPerms,
    channel_id: i64,
) -> AppResult<i64> {
    if perms.role == Role::Owner || has(perms.bits, ADMINISTRATOR) {
        return Ok(ALL);
    }
let overwrites: Vec<OverwriteRow> = channel_overwrites::table
        .filter(channel_overwrites::channel_id.eq(channel_id))
        .select((
            channel_overwrites::role_id,
            channel_overwrites::user_id,
            channel_overwrites::allow,
            channel_overwrites::deny,
        ))
        .load(conn)
        .await?;

    let mut bits = perms.bits;
    for o in overwrites.iter().filter(|o| o.user_id.is_none()) {
        if o.role_id.is_some_and(|id| perms.role_ids.contains(&id)) {
            bits = (bits & !o.deny) | o.allow;
        }
    }
    if let Some(o) = overwrites
        .iter()
        .find(|o| o.user_id == Some(perms.user_id))
    {
        bits = (bits & !o.deny) | o.allow;
    }
    Ok(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_sets_are_disjoint_and_complete() {
        assert_eq!(MEMBER_BASE & ADMINISTRATOR, 0);
        assert_eq!(ADMIN_BASE & ADMINISTRATOR, 0);
        assert_eq!(ADMIN_BASE | ADMINISTRATOR, ALL);
        assert!(has(base_for(Role::Owner), ADMINISTRATOR));
        assert!(has(ALL, MANAGE_ROLES));
        assert!(!has(MEMBER_BASE, MANAGE_ROLES));
    }

    #[test]
    fn deny_then_allow() {
        // Как в Discord: итог = (bits & !deny) | allow.
        let bits = VIEW_CHANNEL | SEND_MESSAGES;
        let o = OverwriteRow {
            role_id: Some(1),
            user_id: None,
            allow: MANAGE_MESSAGES,
            deny: SEND_MESSAGES,
        };
        let after = (bits & !o.deny) | o.allow;
        assert!(has(after, VIEW_CHANNEL));
        assert!(!has(after, SEND_MESSAGES));
        assert!(has(after, MANAGE_MESSAGES));
    }
}
