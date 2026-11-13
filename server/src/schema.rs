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
);

diesel::table! {
    servers (id) {
        id -> Int8,
        name -> Text,
        owner_id -> Int8,
        created_at -> Timestamptz,
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
    }
}

diesel::joinable!(messages -> channels (channel_id));
diesel::joinable!(messages -> users (author_id));

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
