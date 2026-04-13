# Subtask 1.1: Implement Server UDP Socket

## Description
Create `server/src/net/udp.rs` and implement `pub async fn run_udp_server(port: u16, sessions: SessionMap, input_queues: InputQueueMap)` — an async task that binds a UDP socket and loops receiving datagrams, deserializing each as a `ClientUdpMessage`, authenticating the sender via `sessions`, and routing the contained `PlayerInput` to the correct `InputQueueHandle`.

## Layer
`server/` crate — `server/src/net/udp.rs`

## Steps
- [ ] Add to `server/Cargo.toml`: `tokio = { version = "1", features = ["net", "rt-multi-thread", "macros"] }` and `bincode = "2"`.
- [ ] Create `server/src/net/mod.rs` and `server/src/net/udp.rs`, declare them in `server/src/main.rs`.
- [ ] Define type aliases: `type SessionMap = Arc<RwLock<HashMap<SocketAddr, PlayerId>>>` and `type InputQueueMap = Arc<RwLock<HashMap<PlayerId, InputQueueHandle>>>`.
- [ ] Implement `run_udp_server`:
  ```rust
  pub async fn run_udp_server(port: u16, sessions: SessionMap, input_queues: InputQueueMap) {
      let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).await.unwrap();
      let mut buf = [0u8; 1400];
      loop {
          let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
          let data = &buf[..len];
          match bincode::decode_from_slice::<ClientUdpMessage, _>(data, bincode::config::standard()) {
              Ok((msg, _)) => dispatch_udp_message(msg, addr, &sessions, &input_queues).await,
              Err(e) => eprintln!("[udp] malformed datagram from {addr}: {e}"),
          }
      }
  }
  ```
- [ ] Implement `dispatch_udp_message`: look up `addr` in `sessions` to find `PlayerId`, look up `PlayerId` in `input_queues` to find `InputQueueHandle`, call `handle.push(input)`.
- [ ] Implement `pub async fn send_to(socket: &UdpSocket, msg: &ServerUdpMessage, addr: SocketAddr)`: serialize with `bincode`, call `socket.send_to`.
- [ ] Run `cargo check -p server`.

## Acceptance Criteria
- `run_udp_server` compiles as an async function.
- Malformed datagrams are caught and logged; the loop continues.
- `send_to` serializes and dispatches without panicking.
- `cargo check -p server` passes.

## Notes
- The `sessions` map is populated by the TCP reliable channel (Feature 2 Task 2) when a player connects and authenticates. For now, stub it as an empty map.
- Buffer size 1400 bytes is intentional (under Ethernet MTU 1500 minus IP/UDP headers).
- `RwLock` is appropriate: many concurrent readers (per-tick send path), rare writers (connect/disconnect).

## Dependencies
- Feature 2 Task 3 subtask `3_1_define_protocol_message_types` — `ClientUdpMessage`, `ServerUdpMessage` must be defined.
