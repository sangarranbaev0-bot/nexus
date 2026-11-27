ALTER TABLE servers
    ADD COLUMN boost_count INT NOT NULL DEFAULT 0,
    ADD COLUMN boost_level INT NOT NULL DEFAULT 0;

CREATE TABLE server_boosts (
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    boosted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (server_id, user_id)
);
CREATE INDEX server_boosts_server_idx ON server_boosts(server_id);
