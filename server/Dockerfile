# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.87-slim AS builder

WORKDIR /app

# No native libs needed by the server crate; pkg-config satisfies any probes.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy full workspace and build the real server binary.
COPY . .

RUN cargo build --release -p server

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /app/server
COPY assets/ /app/assets/

# Configurable via environment variables (see server/src/main.rs).
ENV SERVER_BIND_ADDR=0.0.0.0:7777
ENV SERVER_UDP_PORT=7777

EXPOSE 7777/udp

CMD ["/app/server"]
