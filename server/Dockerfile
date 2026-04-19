# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.87-slim AS builder

WORKDIR /app

# No native libs needed by the server crate; pkg-config satisfies any probes.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests and lock file first so Docker caches the dependency
# download layer separately from source changes.
COPY Cargo.toml Cargo.lock ./
COPY engine/Cargo.toml engine/Cargo.toml
COPY game/Cargo.toml   game/Cargo.toml
COPY net/Cargo.toml    net/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml
COPY client/Cargo.toml client/Cargo.toml

# Stub every crate with the minimal entry-point it needs so cargo can fetch
# and compile all dependencies without the real sources.
RUN mkdir -p engine/src game/src net/src server/src client/src \
    && printf 'pub fn _stub() {}\n' \
         | tee engine/src/lib.rs game/src/lib.rs net/src/lib.rs net/src/lib.rs > /dev/null \
    && echo 'fn main() {}' | tee server/src/main.rs client/src/main.rs > /dev/null \
    && cargo build --release -p server 2>&1 || true \
    && rm -rf engine/src game/src net/src server/src client/src

# Copy real sources and do the actual build.
COPY engine/ engine/
COPY game/   game/
COPY net/    net/
COPY server/ server/

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
ENV SERVER_BIND_ADDR=0.0.0.0
ENV SERVER_UDP_PORT=7777

EXPOSE 7777/udp

CMD ["/app/server"]
