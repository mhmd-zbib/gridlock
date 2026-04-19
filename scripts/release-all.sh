#!/usr/bin/env bash
# Build releases for all platforms.
#
# Usage:
#   ./scripts/release-all.sh                           # all three platforms
#   ./scripts/release-all.sh mac linux                 # specific platforms
#   ./scripts/release-all.sh --server 1.2.3.4          # bake VPS IP into all clients
#   ./scripts/release-all.sh --server 1.2.3.4:9000 mac # custom port, mac only
set -euo pipefail

SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

SERVER_ARGS=()
PLATFORMS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --server) SERVER_ARGS=(--server "$2"); shift 2 ;;
        *) PLATFORMS+=("$1"); shift ;;
    esac
done

if [ ${#PLATFORMS[@]} -eq 0 ]; then
    PLATFORMS=(mac linux windows)
fi

FAILED=()

for platform in "${PLATFORMS[@]}"; do
    script="$SCRIPTS/release-$platform.sh"
    if [ ! -f "$script" ]; then
        echo "[release-all] unknown platform: $platform (no $script)"
        FAILED+=("$platform")
        continue
    fi

    echo ""
    echo "══════════════════════════════════════════"
    echo " Building: $platform"
    echo "══════════════════════════════════════════"

    if bash "$script" "${SERVER_ARGS[@]}"; then
        echo "[release-all] $platform ✓"
    else
        echo "[release-all] $platform FAILED"
        FAILED+=("$platform")
    fi
done

echo ""
if [ ${#FAILED[@]} -eq 0 ]; then
    echo "[release-all] all platforms built successfully."
    echo "Archives are in: $(cd "$SCRIPTS/.." && pwd)/dist/"
else
    echo "[release-all] failed platforms: ${FAILED[*]}"
    exit 1
fi
