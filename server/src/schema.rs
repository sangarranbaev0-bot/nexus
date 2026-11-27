// Соответствует migrations/. При изменении схемы обновлять вместе с миграцией.
diesel::table! {
    users (id) {
        id -> Int8,
        email -> Text,
        username -> Text,
        created_at -> Timestamptz,
        password_hash -> Text,
        about -> Text,
        banner_url -> Text,
        pronouns -> Text,
        accent_color -> Nullable<Int4>,
        status_text -> Text,
        status_emoji -> Text,
        status_expires_at -> Nullable<Timestamptz>,
        dnd -> Bool,
        totp_secret -> Nullable<Text>,
        two_factor_enabled -> Bool,
        email_verified -> Bool,
        activity_name -> Text,
        activity_type -> Text,
        notify_mode -> Text,
        badge -> Text,
    }
}

diesel::table! {
    auth_accounts (id) {
        id -> Int8,
        user_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(auth_accounts -> users (user_id));

diesel::table! {
    refresh_tokens (id) {
        id -> Int8,
        user_id -> Int8,
        token_hash -> Text,
        expires_at -> Timestamptz,
        revoked_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        user_agent -> Text,
        ip -> Text,
        last_used_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(refresh_tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    users,
    auth_accounts,
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
    reaction_roles,
    thread_tags,
    onboarding_questions,
    onboarding_answers,
    server_events,
    event_interests,
    user_notes,
    recovery_codes,
    friend_requests,
    friends,
    blocks,
    notify_settings,
    notifications,
    notification_words,
    bans,
    automod_rules,
    sticker_packs,
    stickers,
    server_boosts,
    webhooks,
    oauth_apps,
    drafts,
    scheduled_messages,
);

diesel::table! {
    tickets (id) {
        id -> Int8,
        server_id -> Int8,
        creator_id -> Int8,
        status -> Text,
        title -> Text,
        created_at -> Timestamptz,
        closed_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(tickets -> servers (server_id));
diesel::joinable!(tickets -> users (creator_id));

diesel::table! {
    servers (id) {
        id -> Int8,
        name -> Text,
        owner_id -> Int8,
        created_at -> Timestamptz,
        system_channel_id -> Nullable<Int8>,
        afk_channel_id -> Nullable<Int8>,
        description -> Text,
        rules -> Text,
        banner_url -> Text,
        announcement_channel_id -> Nullable<Int8>,
        boost_count -> Int4,
        boost_level -> Int4,
    }
}

diesel::table! {
    server_boosts (server_id, user_id) {
        server_id -> Int8,
        user_id -> Int8,
        boosted_at -> Timestamptz,
    }
}

diesel::joinable!(server_boosts -> servers (server_id));
diesel::joinable!(server_boosts -> users (user_id));

diesel::table! {
    members (server_id, user_id) {
        server_id -> Int8,
        user_id -> Int8,
        role -> Text,
        joined_at -> Timestamptz,
        nickname -> Text,
        avatar_url -> Text,
        communication_disabled_until -> Nullable<Timestamptz>,
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
        is_forum -> Bool,
        topic -> Text,
        slowmode_seconds -> Int4,
        nsfw -> Bool,
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
        forwarded_from_id -> Nullable<Int8>,
        forwarded_channel_id -> Nullable<Int8>,
        forwarded_author_id -> Nullable<Int8>,
        forwarded_author -> Nullable<Text>,
        forwarded_content -> Nullable<Text>,
        thread_root_id -> Nullable<Int8>,
        thread_archived -> Bool,
        thread_locked -> Bool,
        has_embed -> Bool,
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
    server_emojis (id) {
        id -> Int8,
        server_id -> Int8,
        name -> Text,
        content_type -> Text,
        size_bytes -> Int8,
        data -> Bytea,
        created_by -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    thread_tags (id) {
        id -> Int8,
        channel_id -> Int8,
        name -> Text,
        color -> Nullable<Int4>,
    }
}

diesel::table! {
    onboarding_questions (id) {
        id -> Int8,
        server_id -> Int8,
        question -> Text,
        role_id -> Int8,
        position -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    onboarding_answers (question_id, user_id) {
        question_id -> Int8,
        user_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    server_events (id) {
        id -> Int8,
        server_id -> Int8,
        name -> Text,
        description -> Text,
        starts_at -> Timestamptz,
        channel_id -> Nullable<Int8>,
        creator_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    user_notes (owner_id, target_id) {
        owner_id -> Int8,
        target_id -> Int8,
        note -> Text,
        nickname -> Text,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    recovery_codes (user_id, code_hash) {
        user_id -> Int8,
        code_hash -> Text,
        used_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    event_interests (event_id, user_id) {
        event_id -> Int8,
        user_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    reaction_roles (message_id, emoji, role_id) {
        message_id -> Int8,
        emoji -> Text,
        role_id -> Int8,
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

diesel::table! {
    bans (id) {
        id -> Int8,
        server_id -> Int8,
        user_id -> Int8,
        actor_id -> Int8,
        reason -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    automod_rules (id) {
        id -> Int8,
        server_id -> Int8,
        pattern -> Text,
        action -> Text,
        note -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    sticker_packs (id) {
        id -> Int8,
        server_id -> Int8,
        name -> Text,
        description -> Text,
        created_by -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    stickers (id) {
        id -> Int8,
        pack_id -> Int8,
        name -> Text,
        emoji -> Text,
        content_type -> Text,
        size_bytes -> Int8,
        storage_key -> Text,
        tags -> Array<Text>,
        position -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    webhooks (id) {
        id -> Int8,
        channel_id -> Int8,
        name -> Text,
        avatar_url -> Text,
        token -> Text,
        created_by -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    oauth_apps (id) {
        id -> Int8,
        owner_id -> Int8,
        name -> Text,
        client_id -> Text,
        client_secret_hash -> Text,
        scopes -> Array<Text>,
        redirect_uris -> Array<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    drafts (id) {
        id -> Int8,
        user_id -> Int8,
        channel_id -> Int8,
        content -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    scheduled_messages (id) {
        id -> Int8,
        author_id -> Int8,
        channel_id -> Int8,
        content -> Text,
        scheduled_at -> Timestamptz,
        status -> Text,
        sent_at -> Nullable<Timestamptz>,
        error_msg -> Nullable<Text>,
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

diesel::joinable!(automod_rules -> servers (server_id));
diesel::joinable!(sticker_packs -> servers (server_id));
diesel::joinable!(sticker_packs -> users (created_by));
diesel::joinable!(stickers -> sticker_packs (pack_id));
diesel::joinable!(webhooks -> channels (channel_id));
diesel::joinable!(webhooks -> users (created_by));
diesel::joinable!(oauth_apps -> users (owner_id));
diesel::joinable!(drafts -> users (user_id));
diesel::joinable!(drafts -> channels (channel_id));
diesel::joinable!(scheduled_messages -> users (author_id));
diesel::joinable!(scheduled_messages -> channels (channel_id));