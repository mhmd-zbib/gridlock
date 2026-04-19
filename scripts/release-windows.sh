#!/usr/bin/env bash
# Build a Windows x86_64 release via the MinGW cross-compiler.
#
# Usage:
#   ./scripts/release-windows.sh                        # connects to 127.0.0.1:7777 (dev)
#   ./scripts/release-windows.sh --server 1.2.3.4       # port defaults to 7777
#   ./scripts/release-windows.sh --server 1.2.3.4:9000  # custom port
#
# Prerequisites (macOS):
#   brew install mingw-w64
#   rustup target add x86_64-pc-windows-gnu
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep '^version' "$ROOT/Cargo.toml" | head -1 | grep -o '".*"' | tr -d '"')"
TARGET="x86_64-pc-windows-gnu"
DIST="$ROOT/dist/windows"

# ── Parse arguments ───────────────────────────────────────────────────────────

SERVER_HOST=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --server) SERVER_HOST="$2"; shift 2 ;;
        *) echo "[release-windows] unknown argument: $1"; exit 1 ;;
    esac
done

if [[ -n "$SERVER_HOST" && "$SERVER_HOST" != *:* ]]; then
    SERVER_HOST="$SERVER_HOST:7777"
fi

if [[ -n "$SERVER_HOST" ]]; then
    export SERVER_ADDR="$SERVER_HOST"
    echo "[release-windows] version $VERSION  target $TARGET  server $SERVER_ADDR"
else
    echo "[release-windows] version $VERSION  target $TARGET  server 127.0.0.1:7777 (dev default)"
fi

# ── Dependency checks ─────────────────────────────────────────────────────────

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "[release-windows] MinGW not found. Install it with: brew install mingw-w64"
    exit 1
fi

if ! rustup target list --installed | grep -q "$TARGET"; then
    echo "[release-windows] adding rustup target $TARGET"
    rustup target add "$TARGET"
fi

# ── Build ─────────────────────────────────────────────────────────────────────

cd "$ROOT"

# Tell cargo to use the MinGW linker for this target.
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="x86_64-w64-mingw32-gcc"

echo "[release-windows] building client..."
cargo build --release -p client --target "$TARGET"

echo "[release-windows] building server..."
cargo build --release -p server --target "$TARGET"

# ── Package ───────────────────────────────────────────────────────────────────

mkdir -p "$DIST"

cp "target/$TARGET/release/client.exe" "$DIST/client.exe"
cp "target/$TARGET/release/server.exe" "$DIST/server.exe"
cp -r "$ROOT/assets" "$DIST/assets"

ARCHIVE="$ROOT/dist/shooting-$VERSION-windows.zip"
cd "$ROOT/dist"
zip -r "$ARCHIVE" windows

echo "[release-windows] done → $ARCHIVE"
