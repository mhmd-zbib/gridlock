#!/usr/bin/env bash
# Build a Linux x86_64 release using `cross` (Docker-based cross compiler).
#
# Usage:
#   ./scripts/release-linux.sh                        # connects to 127.0.0.1:7777 (dev)
#   ./scripts/release-linux.sh --server 1.2.3.4       # port defaults to 7777
#   ./scripts/release-linux.sh --server 1.2.3.4:9000  # custom port
#
# Prerequisites:
#   cargo install cross
#   Docker running
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep '^version' "$ROOT/server/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')"
TARGET="x86_64-unknown-linux-gnu"
DIST="$ROOT/dist/linux"

# ── Parse arguments ───────────────────────────────────────────────────────────

SERVER_HOST=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --server) SERVER_HOST="$2"; shift 2 ;;
        *) echo "[release-linux] unknown argument: $1"; exit 1 ;;
    esac
done

if [[ -n "$SERVER_HOST" && "$SERVER_HOST" != *:* ]]; then
    SERVER_HOST="$SERVER_HOST:7777"
fi

if [[ -n "$SERVER_HOST" ]]; then
    export SERVER_ADDR="$SERVER_HOST"
    echo "[release-linux] version $VERSION  target $TARGET  server $SERVER_ADDR"
else
    echo "[release-linux] version $VERSION  target $TARGET  server 127.0.0.1:7777 (dev default)"
fi

# ── Dependency checks ─────────────────────────────────────────────────────────

if ! command -v cross &>/dev/null; then
    echo "[release-linux] 'cross' not found. Install it with: cargo install cross"
    exit 1
fi

if ! docker info &>/dev/null; then
    echo "[release-linux] Docker is not running. cross requires Docker."
    exit 1
fi

# ── Build ─────────────────────────────────────────────────────────────────────

cd "$ROOT"

echo "[release-linux] building client..."
cross build --release -p client --target "$TARGET"

echo "[release-linux] building server..."
cross build --release -p server --target "$TARGET"

# ── Package ───────────────────────────────────────────────────────────────────

mkdir -p "$DIST"

cp "target/$TARGET/release/client" "$DIST/client"
cp "target/$TARGET/release/server" "$DIST/server"
chmod +x "$DIST/client" "$DIST/server"

cp -r "$ROOT/assets" "$DIST/assets"

ARCHIVE="$ROOT/dist/shooting-$VERSION-linux.tar.gz"
tar -czf "$ARCHIVE" -C "$ROOT/dist" linux

echo "[release-linux] done → $ARCHIVE"
