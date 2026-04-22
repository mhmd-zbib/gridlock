#!/usr/bin/env bash
# Build a Linux x86_64 release using musl (no Docker required).
#
# Usage:
#   ./scripts/release-linux.sh                        # SERVER_ADDR defaults to 127.0.0.1:7777 at runtime
#   ./scripts/release-linux.sh --server 1.2.3.4       # port defaults to 7777
#   ./scripts/release-linux.sh --server 1.2.3.4:9000  # custom port
#
# The server address is set via the SERVER_ADDR env var at runtime (not baked in at compile time).
# When --server is provided a play.sh launcher is generated in the dist folder.
#
# Prerequisites (macOS):
#   brew install FiloSottile/musl-cross/musl-cross
#   rustup target add x86_64-unknown-linux-musl
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep '^version' "$ROOT/server/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')"
TARGET="x86_64-unknown-linux-musl"
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
    echo "[release-linux] version $VERSION  target $TARGET  server $SERVER_HOST (runtime)"
else
    echo "[release-linux] version $VERSION  target $TARGET  server 127.0.0.1:7777 (runtime default)"
fi

# ── Dependency checks ─────────────────────────────────────────────────────────

if ! command -v x86_64-linux-musl-gcc &>/dev/null; then
    echo "[release-linux] musl cross-compiler not found. Install with:"
    echo "  brew install FiloSottile/musl-cross/musl-cross"
    exit 1
fi

if ! rustup target list --installed | grep -q "$TARGET"; then
    echo "[release-linux] adding rustup target $TARGET"
    rustup target add "$TARGET"
fi

# ── Build ─────────────────────────────────────────────────────────────────────

cd "$ROOT"

export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="x86_64-linux-musl-gcc"

echo "[release-linux] building client..."
cargo build --release -p client --target "$TARGET"

echo "[release-linux] building server..."
cargo build --release -p server --target "$TARGET"

# ── Package ───────────────────────────────────────────────────────────────────

mkdir -p "$DIST"

cp "target/$TARGET/release/client" "$DIST/client"
cp "target/$TARGET/release/server" "$DIST/server"
chmod +x "$DIST/client" "$DIST/server"

cp -r "$ROOT/assets" "$DIST/assets"

# Generate a launcher that injects SERVER_ADDR at runtime.
LAUNCHER_ADDR="${SERVER_HOST:-127.0.0.1:7777}"
cat > "$DIST/play.sh" <<EOF
#!/usr/bin/env bash
SERVER_ADDR="${LAUNCHER_ADDR}" "\$(dirname "\$0")/client" "\$@"
EOF
chmod +x "$DIST/play.sh"

ARCHIVE="$ROOT/dist/gridlock-$VERSION-linux.tar.gz"
tar -czf "$ARCHIVE" -C "$ROOT/dist" linux

echo "[release-linux] done → $ARCHIVE"
