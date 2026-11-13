# Образ сервера NexusChat: Rust-сервер + собранный React-клиент, который он сам отдаёт.

FROM node:22-bookworm-slim AS client
WORKDIR /app/client
RUN corepack enable
COPY client/package.json client/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY client/ ./
RUN pnpm build

FROM rust:1-bookworm AS server
WORKDIR /app/server
COPY server/ ./
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 nexus
COPY --from=server /app/server/target/release/nexus-server /usr/local/bin/nexus-server
COPY --from=client /app/client/dist /app/client
ENV CLIENT_DIST=/app/client PORT=3001
USER nexus
EXPOSE 3001
CMD ["nexus-server"]
