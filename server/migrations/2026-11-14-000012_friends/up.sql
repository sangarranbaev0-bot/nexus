CREATE TABLE friend_requests (
    id BIGSERIAL PRIMARY KEY,
    from_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    to_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT friend_requests_not_self CHECK (from_user_id <> to_user_id),
    CONSTRAINT friend_requests_pair_unique UNIQUE (from_user_id, to_user_id)
);

CREATE INDEX friend_requests_to_idx ON friend_requests(to_user_id);

CREATE TABLE friends (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    friend_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, friend_id),
    CONSTRAINT friends_not_self CHECK (user_id <> friend_id)
);

CREATE INDEX friends_friend_idx ON friends(friend_id);

CREATE TABLE blocks (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, blocked_id),
    CONSTRAINT blocks_not_self CHECK (user_id <> blocked_id)
);

CREATE INDEX blocks_blocked_idx ON blocks(blocked_id);
