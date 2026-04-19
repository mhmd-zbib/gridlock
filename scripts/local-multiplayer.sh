#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLIENT_COUNT="${1:-2}"

if ! [[ "$CLIENT_COUNT" =~ ^[0-9]+$ ]] || [ "$CLIENT_COUNT" -lt 1 ]; then
  echo "Usage: $0 [client_count>=1]"
  exit 1
fi

cd "$ROOT_DIR"

echo "[local-multiplayer] Building server + client..."
cargo build -p server -p client

PIDS=()

cleanup() {
  echo
  echo "[local-multiplayer] Stopping processes..."
  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then-+-+
      kill "$pid"
    fi
  done
  wait || true
}

trap cleanup EXIT INT TERM

echo "[local-multiplayer] Starting server..."
"$ROOT_DIR/target/debug/server" &
PIDS+=("$!")

sleep 1

for i in $(seq 1 "$CLIENT_COUNT"); do
  echo "[local-multiplayer] Starting client $i..."
  "$ROOT_DIR/target/debug/client" &
  PIDS+=("$!")
  sleep 1
done

echo "[local-multiplayer] Running with $CLIENT_COUNT client(s). Press Ctrl+C to stop all."
wait
