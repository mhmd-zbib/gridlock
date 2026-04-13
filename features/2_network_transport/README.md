# Feature 2: Network Transport Layer

## Overview
The transport layer is the wire boundary between server and clients. It implements two channels: an unreliable UDP channel for high-frequency game inputs and state snapshots (hot path), and a reliable ordered channel for connection handshakes, authentication, critical events, and chat (control path). The design deliberately avoids pure TCP for game data to prevent head-of-line blocking. This feature does not implement game logic — it only moves bytes reliably or unreliably between endpoints.

## Tools & Context
- **`tokio`**: Async runtime for the server's UDP socket and reliable channel tasks. Add to `server/Cargo.toml` and `client/Cargo.toml`.
- **`tokio::net::UdpSocket`**: Non-blocking UDP socket for the hot path.
- **`tokio::net::TcpListener` / `TcpStream`**: Reliable control channel. A TCP stream per connected client, framed with a 4-byte length prefix.
- **`bincode`** (or `serde_json` as fallback): Binary serialization for UDP packets to minimize payload size. Add `bincode = "2"` to workspace dependencies.
- **Existing `game/src/sim/input.rs`**: `PlayerInput` is the primary payload on the UDP uplink (client → server).
- **Feature 1 snapshots**: `GameState` / delta packets are the primary payload on the UDP downlink (server → client).

## Layer Placement
- `server/` crate: Owns UDP socket binding, TCP listener, per-client connection state, packet dispatch.
- `client/` crate: Owns UDP socket (client-side), TCP stream to server, send/receive loops.
- `game/` crate: Defines all packet payload types (`PlayerInput`, snapshot structs) — no socket code.

## Tasks
1. [ ] UDP Socket Layer – Implement server UDP socket that receives `PlayerInput` packets and dispatches them to the input queue, and sends serialized packets to client addresses.
2. [ ] Reliable Channel – Implement length-prefixed TCP framing for the control path on both server and client sides.
3. [ ] Protocol Message Types – Define all wire message enums (`ClientMessage`, `ServerMessage`) with serialization, covering both UDP and TCP channels.

## Dependencies
- Feature 1 (`simulation_isolation`) — `PlayerInput` and `GameState` must exist before protocol messages can reference them.
