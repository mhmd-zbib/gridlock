# Task 1: UDP Socket Layer

## Description
Implement the UDP socket infrastructure on both server and client. The server binds a UDP socket, receives datagrams, deserializes them as `ClientUdpMessage`, routes inputs to the appropriate `InputQueueHandle`, and sends serialized `ServerUdpMessage` datagrams back to registered client addresses. The client binds its own UDP socket, sends `PlayerInput` packets, and receives server datagrams for the snapshot system.

## Layer
`server/` crate — `server/src/net/udp.rs`
`client/` crate — `client/src/net/udp.rs`

## Dependencies
- Feature 2 Task 3 (`3_protocol_message_types`) — `ClientUdpMessage` and `ServerUdpMessage` must be defined.
- Feature 1 Task 1 (`1_deterministic_simulation_core`) — `PlayerInput` must exist.

## Acceptance Criteria
- Server binds `0.0.0.0:7777` UDP and receives datagrams in an async task loop.
- Each received datagram is deserialized; on success the contained `PlayerInput` is pushed to the matching `InputQueueHandle`.
- Server can send a serialized datagram to any `SocketAddr` via `send_to`.
- Client binds a local UDP socket and can `send_to` the server address.
- Malformed datagrams are logged and discarded without crashing.
- Maximum datagram size enforced at 1400 bytes (under Ethernet MTU); packets larger than 1400 bytes are discarded and logged.
- `cargo build --workspace` succeeds.

## Notes
- Use `tokio::net::UdpSocket` on both server and client.
- The server maintains a `HashMap<PlayerId, SocketAddr>` mapping to know where to send per-client datagrams. This map is populated when a player authenticates via the reliable channel (Feature 2 Task 2).
- `bincode::encode_to_vec` / `bincode::decode_from_slice` for (de)serialization. Fall back to `serde_json` if `bincode` v2 API is unfamiliar — correctness first.
- Do not implement retransmission on the UDP path — that is the reliable channel's job.

## Subtasks
1. `1_1_implement_server_udp_socket` – Bind server UDP socket and implement the async receive loop.
2. `1_2_test_server_udp_receive` – Write an integration test that sends a UDP datagram to the server socket and asserts it is dispatched to the input queue.
3. `1_3_implement_client_udp_socket` – Implement client UDP socket with send and async receive loops.
4. `1_4_test_client_udp_send_receive` – Write a loopback test confirming the client can send a `PlayerInput` packet and receive a response datagram.
