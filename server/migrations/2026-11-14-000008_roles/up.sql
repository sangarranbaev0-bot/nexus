-- Роли, назначение ролей, категории каналов, переопределения прав и журнал аудита.
-- Права хранятся битовой маской; биты описаны в server/src/permissions.rs.

CREATE TABLE roles (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    color       INT,
    permissions BIGINT NOT NULL DEFAULT 0,
    position    INT NOT NULL,
    hoist       BOOLEAN NOT NULL DEFAULT FALSE,
    mentionable BOOLEAN NOT NULL DEFAULT FALSE,
    is_default  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT roles_name_len CHECK (char_length(name) BETWEEN 1 AND 100),
    CONSTRAINT roles_color_range CHECK (color IS NULL OR color BETWEEN 0 AND 16777215),
    CONSTRAINT roles_position_range CHECK (position BETWEEN 0 AND 1000),
    UNIQUE (server_id, name)
);
CREATE UNIQUE INDEX roles_one_default_idx ON roles(server_id) WHERE is_default;
CREATE INDEX roles_server_id_idx ON roles(server_id);

-- @everyone для уже существующих серверов. Новые серверы получают её при создании.
INSERT INTO roles (server_id, name, permissions, position, is_default)
SELECT id, '@everyone', 0, 0, TRUE FROM servers;

CREATE TABLE member_roles (
    server_id BIGINT NOT NULL,
    user_id   BIGINT NOT NULL,
    role_id   BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (server_id, user_id, role_id),
    FOREIGN KEY (server_id, user_id) REFERENCES members(server_id, user_id) ON DELETE CASCADE
);
CREATE INDEX member_roles_role_id_idx ON member_roles(role_id);

CREATE TABLE categories (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    server_id  BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT categories_name_len CHECK (char_length(name) BETWEEN 1 AND 100),
    CONSTRAINT categories_position_range CHECK (position BETWEEN 0 AND 1000)
);
CREATE INDEX categories_server_id_idx ON categories(server_id);

ALTER TABLE channels ADD COLUMN category_id BIGINT REFERENCES categories(id) ON DELETE SET NULL;
CREATE INDEX channels_category_id_idx ON channels(category_id);

-- Переопределения прав на канал. Ровно одна цель: роль или участник.
CREATE TABLE channel_overwrites (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    role_id    BIGINT REFERENCES roles(id) ON DELETE CASCADE,
    user_id    BIGINT REFERENCES users(id) ON DELETE CASCADE,
    allow      BIGINT NOT NULL DEFAULT 0,
    deny       BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT channel_overwrites_one_target CHECK ((role_id IS NOT NULL) <> (user_id IS NOT NULL))
);
CREATE UNIQUE INDEX channel_overwrites_role_idx
    ON channel_overwrites(channel_id, role_id) WHERE role_id IS NOT NULL;
CREATE UNIQUE INDEX channel_overwrites_user_idx
    ON channel_overwrites(channel_id, user_id) WHERE user_id IS NOT NULL;

CREATE TABLE audit_log (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    server_id  BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    actor_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action     TEXT NOT NULL,
    target     TEXT,
    metadata   JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX audit_log_server_id_idx ON audit_log(server_id, created_at DESC);