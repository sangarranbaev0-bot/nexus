// Соответствует migrations/. При изменении схемы обновлять вместе с миграцией.
diesel::table! {
    users (id) {
        id -> Int8,
        email -> Text,
        username -> Text,
        created_at -> Timestamptz,
        password_hash -> Text,
    }
}

diesel::table! {
    refresh_tokens (id) {
        id -> Int8,
        user_id -> Int8,
        token_hash -> Text,
        expires_at -> Timestamptz,
        revoked_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(refresh_tokens -> users (user_id));
diesel::allow_tables_to_appear_in_same_query!(
    users,
    refresh_tokens,
    servers,
    members,
    channels,
    invites,
    messages,
    read_states,
    presence_connections,
    dm_participants,
    attachments,
    voice_states,
    voice_signals,
    roles,
    member_roles,
    categories,
    channel_overwrites,
    audit_log,
    friend_requests,
    friends,
    blocks,
    notify_settings,
    notifications,
    notification_words,
);

diesel::table! {
    servers (id) {
        id -> Int8,
        name -> Text,
        owner_id -> Int8,
        created_at -> Timestamptz,
        system_channel_id -> Nullable<Int8>,
    }
}

diesel::table! {
    members (server_id, user_id) {
        server_id -> Int8,
        user_id -> Int8,
        role -> Text,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    channels (id) {
        id -> Int8,
        server_id -> Nullable<Int8>,
        name -> Text,
        position -> Int4,
        created_at -> Timestamptz,
        dm_key -> Nullable<Text>,
        kind -> Text,
        category_id -> Nullable<Int8>,
    }
}

diesel::table! {
    invites (code) {
        code -> Text,
        server_id -> Int8,
        created_by -> Int8,
        expires_at -> Nullable<Timestamptz>,
        max_uses -> Nullable<Int4>,
        uses -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(members -> servers (server_id));
diesel::joinable!(members -> users (user_id));
diesel::joinable!(channels -> servers (server_id));
diesel::joinable!(invites -> servers (server_id));

diesel::table! {
    messages (id) {
        id -> Int8,
        channel_id -> Int8,
        author_id -> Int8,
        content -> Text,
        created_at -> Timestamptz,
        edited_at -> Nullable<Timestamptz>,
        reply_to_id -> Nullable<Int8>,
        pinned_at -> Nullable<Timestamptz>,
        system_kind -> Nullable<Text>,
        system_detail -> Nullable<Text>,
    }
}

diesel::joinable!(messages -> channels (channel_id));
diesel::joinable!(messages -> users (author_id));
diesel::joinable!(message_edits -> messages (message_id));

diesel::table! {
    message_reactions (message_id, user_id, emoji) {
        message_id -> Int8,
        user_id -> Int8,
        emoji -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(message_reactions -> messages (message_id));
diesel::joinable!(message_reactions -> users (user_id));

diesel::table! {
    read_states (user_id, channel_id) {
        user_id -> Int8,
        channel_id -> Int8,
        last_read_message_id -> Int8,
    }
}

diesel::table! {
    presence_connections (id) {
        id -> Int8,
        user_id -> Int8,
        idle -> Bool,
        heartbeat_at -> Timestamptz,
    }
}

diesel::table! {
    dm_participants (channel_id, user_id) {
        channel_id -> Int8,
        user_id -> Int8,
    }
}

diesel::table! {
    friend_requests (id) {
        id -> Int8,
        from_user_id -> Int8,
        to_user_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    friends (user_id, friend_id) {
        user_id -> Int8,
        friend_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    blocks (user_id, blocked_id) {
        user_id -> Int8,
        blocked_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    message_edits (id) {
        id -> Int8,
        message_id -> Int8,
        content -> Text,
        edited_at -> Timestamptz,
    }
}

diesel::table! {
    notify_settings (user_id, server_id, channel_id) {
        user_id -> Int8,
        server_id -> Nullable<Int8>,
        channel_id -> Nullable<Int8>,
        level -> Text,
        muted_until -> Nullable<Timestamptz>,
        suppress_everyone -> Bool,
    }
}

diesel::table! {
    notifications (id) {
        id -> Int8,
        user_id -> Int8,
        kind -> Text,
        server_id -> Nullable<Int8>,
        channel_id -> Int8,
        message_id -> Int8,
        actor_id -> Int8,
        created_at -> Timestamptz,
        read_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    notification_words (user_id, word) {
        user_id -> Int8,
        word -> Text,
        case_sensitive -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attachments (id) {
        id -> Int8,
        channel_id -> Nullable<Int8>,
        message_id -> Nullable<Int8>,
        uploader_id -> Int8,
        filename -> Text,
        content_type -> Text,
        size_bytes -> Int8,
        storage_key -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    voice_states (user_id) {
        user_id -> Int8,
        channel_id -> Int8,
        connection_id -> Int8,
        muted -> Bool,
        deafened -> Bool,
        camera -> Bool,
        screen -> Bool,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    voice_signals (id) {
        id -> Int8,
        from_user -> Int8,
        to_user -> Int8,
        payload -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    roles (id) {
        id -> Int8,
        server_id -> Int8,
        name -> Text,
        color -> Nullable<Int4>,
        permissions -> Int8,
        position -> Int4,
        hoist -> Bool,
        mentionable -> Bool,
        is_default -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    member_roles (server_id, user_id, role_id) {
        server_id -> Int8,
        user_id -> Int8,
        role_id -> Int8,
    }
}

diesel::table! {
    categories (id) {
        id -> Int8,
        server_id -> Int8,
        name -> Text,
        position -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    channel_overwrites (id) {
        id -> Int8,
        channel_id -> Int8,
        role_id -> Nullable<Int8>,
        user_id -> Nullable<Int8>,
        allow -> Int8,
        deny -> Int8,
    }
}

diesel::table! {
    audit_log (id) {
        id -> Int8,
        server_id -> Int8,
        actor_id -> Int8,
        action -> Text,
        target -> Nullable<Text>,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(roles -> servers (server_id));
diesel::joinable!(member_roles -> roles (role_id));
diesel::joinable!(categories -> servers (server_id));
diesel::joinable!(channel_overwrites -> channels (channel_id));
diesel::joinable!(channel_overwrites -> roles (role_id));
diesel::joinable!(channel_overwrites -> users (user_id));
diesel::joinable!(audit_log -> servers (server_id));
diesel::joinable!(audit_log -> users (actor_id));
