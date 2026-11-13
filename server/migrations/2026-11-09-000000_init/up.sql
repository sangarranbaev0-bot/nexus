-- Поля для аутентификации (пароль, аватар) добавит этап 1.
CREATE TABLE users (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email      TEXT NOT NULL UNIQUE,
    username   TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
