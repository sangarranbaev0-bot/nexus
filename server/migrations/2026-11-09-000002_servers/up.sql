CREATE TABLE servers (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name       TEXT NOT NULL,
    owner_id   BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE members (
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role      TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (server_id, user_id)
);
CREATE INDEX members_user_id_idx ON members(user_id);

CREATE TABLE channels (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    server_id  BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    position   INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX channels_server_id_idx ON channels(server_id);

CREATE TABLE invites (
    code       TEXT PRIMARY KEY,
    server_id  BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ,
    max_uses   INT,
    uses       INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
