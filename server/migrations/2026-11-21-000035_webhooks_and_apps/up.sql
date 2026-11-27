CREATE TABLE webhooks (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    avatar_url TEXT NOT NULL DEFAULT '',
    token      TEXT NOT NULL UNIQUE,
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX webhooks_channel_idx ON webhooks(channel_id);

CREATE TABLE oauth_apps (
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner_id          BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    client_id         TEXT NOT NULL UNIQUE,
    client_secret_hash TEXT NOT NULL,
    scopes            TEXT[] NOT NULL DEFAULT '{}',
    redirect_uris     TEXT[] NOT NULL DEFAULT '{}',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
