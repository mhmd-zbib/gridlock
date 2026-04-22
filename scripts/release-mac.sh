#!/usr/bin/env bash
# Build a universal macOS release (arm64 + x86_64 client & server).
#
# Usage:
#   ./scripts/release-mac.sh                        # SERVER_ADDR defaults to 127.0.0.1:7777 at runtime
#   ./scripts/release-mac.sh --server 1.2.3.4       # port defaults to 7777
#   ./scripts/release-mac.sh --server 1.2.3.4:9000  # custom port
#
# The server address is set via the SERVER_ADDR env var at runtime (not baked in at compile time).
# When --server is provided a play.sh launcher is generated in the dist folder.
#
# Prerequisites:
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep '^version' "$ROOT/server/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')"
DIST="$ROOT/dist/mac"

# ── Parse arguments ───────────────────────────────────────────────────────────

SERVER_HOST=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --server) SERVER_HOST="$2"; shift 2 ;;
        *) echo "[release-mac] unknown argument: $1"; exit 1 ;;
    esac
done

if [[ -n "$SERVER_HOST" && "$SERVER_HOST" != *:* ]]; then
    SERVER_HOST="$SERVER_HOST:7777"
fi

if [[ -n "$SERVER_HOST" ]]; then
    echo "[release-mac] version $VERSION  server $SERVER_HOST (runtime)"
else
    echo "[release-mac] version $VERSION  server 127.0.0.1:7777 (runtime default)"
fi

# ── Targets ───────────────────────────────────────────────────────────────────

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    if ! rustup target list --installed | grep -q "$target"; then
        echo "[release-mac] adding rustup target $target"
        rustup target add "$target"
    fi
done

cd "$ROOT"

echo "[release-mac] building client (arm64)..."
cargo build --release -p client --target aarch64-apple-darwin

echo "[release-mac] building client (x86_64)..."
cargo build --release -p client --target x86_64-apple-darwin

echo "[release-mac] building server (arm64)..."
cargo build --release -p server --target aarch64-apple-darwin

echo "[release-mac] building server (x86_64)..."
cargo build --release -p server --target x86_64-apple-darwin

# ── Universal binaries ────────────────────────────────────────────────────────

mkdir -p "$DIST"

lipo -create \
    target/aarch64-apple-darwin/release/client \
    target/x86_64-apple-darwin/release/client \
    -output "$DIST/client"

lipo -create \
    target/aarch64-apple-darwin/release/server \
    target/x86_64-apple-darwin/release/server \
    -output "$DIST/server"

chmod +x "$DIST/client" "$DIST/server"

# ── Assets + launcher + archive ───────────────────────────────────────────────

cp -r "$ROOT/assets" "$DIST/assets"

# Generate a launcher that injects SERVER_ADDR at runtime.
LAUNCHER_ADDR="${SERVER_HOST:-127.0.0.1:7777}"
cat > "$DIST/play.sh" <<EOF
#!/usr/bin/env bash
SERVER_ADDR="${LAUNCHER_ADDR}" "\$(dirname "\$0")/client" "\$@"
EOF
chmod +x "$DIST/play.sh"

ARCHIVE="$ROOT/dist/gridlock-$VERSION-mac.tar.gz"
tar -czf "$ARCHIVE" -C "$ROOT/dist" mac

echo "[release-mac] done → $ARCHIVE"
