# NexusChat: Rust server + React client, CA certs from curlimages/curl.
# node:22-slim unreachable from this network — skip client build stage,
# copy pre-built dist/ from host instead.

FROM nexuschat-rust AS server
# image already built locally; contains release binary at /app/server/target/release/nexus-server

FROM curlimages/curl:8.14.1 AS ca-certs

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=ca-certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
RUN useradd --system --uid 10001 nexus
COPY --from=server /app/server/target/release/nexus-server /usr/local/bin/nexus-server
COPY client-bundle/dist /app/client
ENV CLIENT_DIST=/app/client PORT=3001
USER nexus
EXPOSE 3001
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 CMD curl -sf http://localhost:3001/api/health || exit 1
CMD ["nexus-server"]
