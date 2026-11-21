# Образ сервера NexusChat: Rust-сервер + собранный React-клиент, который он сам отдаёт.

FROM node:22-slim AS client
WORKDIR /app/client
RUN corepack enable && corepack prepare pnpm@latest --activate
COPY client/package.json client/pnpm-lock.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
COPY client/ ./
RUN pnpm build

FROM rust:1.89-slim AS server
WORKDIR /app/server
COPY server/ ./
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 nexus
COPY --from=server /app/server/target/release/nexus-server /usr/local/bin/nexus-server
COPY --from=client /app/client/dist /app/client
ENV CLIENT_DIST=/app/client PORT=3001
USER nexus
EXPOSE 3001
CMD ["nexus-server"]
