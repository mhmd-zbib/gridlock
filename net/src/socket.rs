//! UDP socket abstraction.
//!
//! [`NetSocket`] wraps [`tokio::net::UdpSocket`] and provides typed
//! send / receive helpers that work directly with [`AnyPacket`].
//!
//! The server binds one socket and calls [`NetSocket::recv`] in a loop.
//! The client binds an ephemeral port and calls [`NetSocket::send_to`] /
//! [`NetSocket::recv`] on the same socket.

use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::{
    codec::{self, DecodeError},
    proto::handshake::AnyPacket,
};

/// Maximum UDP payload we will ever read or write.
///
/// Keeping this below the typical 1 500 B Ethernet MTU minus IP/UDP headers
/// prevents fragmentation.
pub const MAX_DATAGRAM: usize = 1_400;

// ── NetSocket ─────────────────────────────────────────────────────────────────

/// Thin, async-safe wrapper around a bound UDP socket.
///
/// Obtain one via [`NetSocket::bind`]; then:
/// - **Server**: `recv` in a loop; `send_to` individual clients.
/// - **Client**: `send_to` the server address; `recv` for server replies.
pub struct NetSocket {
    inner: UdpSocket,
}

impl NetSocket {
    /// Bind to `addr`.  Uses `SO_REUSEADDR` on all platforms via Tokio.
    pub async fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let inner = UdpSocket::bind(addr).await?;
        Ok(Self { inner })
    }

    /// The local address this socket is bound to.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    // ── Send ─────────────────────────────────────────────────────────────────

    /// Encode `packet` and transmit it to `addr`.
    ///
    /// Returns the number of bytes sent on success.
    pub async fn send_to(&self, packet: &AnyPacket, addr: SocketAddr) -> std::io::Result<usize> {
        let buf = codec::encode(packet);
        self.inner.send_to(&buf, addr).await
    }

    /// Send a pre-encoded byte slice to `addr` (useful when the caller has
    /// already serialised the packet to avoid redundant work on broadcast).
    pub async fn send_raw(&self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.inner.send_to(buf, addr).await
    }

    // ── Receive ───────────────────────────────────────────────────────────────

    /// Wait for the next datagram, decode it, and return the parsed packet
    /// together with the sender's address.
    ///
    /// Datagrams that exceed [`MAX_DATAGRAM`] or fail to decode are silently
    /// dropped and the call returns [`RecvError::Decode`] / [`RecvError::Oversized`].
    pub async fn recv(&self) -> Result<(AnyPacket, SocketAddr), RecvError> {
        let mut buf = [0u8; MAX_DATAGRAM];
        let (len, addr) = self.inner.recv_from(&mut buf).await?;
        if len > MAX_DATAGRAM {
            return Err(RecvError::Oversized(len));
        }
        let packet = codec::decode(&buf[..len])?;
        Ok((packet, addr))
    }

    /// Receive a raw datagram without decoding it.
    ///
    /// Returns the filled slice of `buf` and the sender address.
    /// The caller is responsible for calling [`codec::decode`].
    pub async fn recv_raw<'b>(
        &self,
        buf: &'b mut [u8; MAX_DATAGRAM],
    ) -> std::io::Result<(usize, SocketAddr)> {
        self.inner.recv_from(buf).await
    }
}

// ── RecvError ─────────────────────────────────────────────────────────────────

/// Errors that can occur when receiving a packet.
#[derive(Debug)]
pub enum RecvError {
    /// Underlying I/O error (socket closed, OS error, etc.).
    Io(std::io::Error),
    /// The datagram exceeds [`MAX_DATAGRAM`]; contains actual length.
    Oversized(usize),
    /// The datagram could not be decoded.
    Decode(DecodeError),
}

impl From<std::io::Error> for RecvError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<DecodeError> for RecvError {
    fn from(e: DecodeError) -> Self {
        Self::Decode(e)
    }
}

impl std::fmt::Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Oversized(n) => write!(f, "datagram too large: {n} bytes"),
            Self::Decode(e) => write!(f, "decode error: {e}"),
        }
    }
}

impl std::error::Error for RecvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Decode(e) => Some(e),
            Self::Oversized(_) => None,
        }
    }
}

// ── Handshake helpers ─────────────────────────────────────────────────────────

use crate::proto::handshake::{ConnectAck, ConnectRequest, ConnectResult, PROTOCOL_VERSION};

impl NetSocket {
    /// **Server-side** — process a [`ConnectRequest`] from `addr`.
    ///
    /// Returns the [`ConnectAck`] that was sent back to the client.  The caller
    /// decides `player_id`, `tick_rate`, and whether the server is full.
    pub async fn accept(
        &self,
        req: &ConnectRequest,
        addr: SocketAddr,
        player_id: u16,
        tick_rate: u8,
        server_time: u32,
    ) -> std::io::Result<ConnectAck> {
        let result = if req.version != PROTOCOL_VERSION {
            ConnectResult::VersionMismatch
        } else {
            ConnectResult::Ok
        };
        let ack = ConnectAck {
            result,
            player_id,
            tick_rate,
            server_time,
        };
        self.send_to(&AnyPacket::ConnectAck(ack), addr).await?;
        Ok(ack)
    }

    /// **Client-side** — send a [`ConnectRequest`] to `server` and wait for a
    /// [`ConnectAck`].
    ///
    /// Returns the ack on success, or an error if the reply is not a
    /// `ConnectAck` or the socket fails.
    pub async fn connect(
        &self,
        server: SocketAddr,
        name: &str,
    ) -> Result<ConnectAck, RecvError> {
        let req = ConnectRequest::new(name);
        self.send_to(&AnyPacket::ConnectRequest(req), server)
            .await
            .map_err(RecvError::Io)?;
        let (packet, _) = self.recv().await?;
        match packet {
            AnyPacket::ConnectAck(ack) => Ok(ack),
            _ => Err(RecvError::Decode(DecodeError::InvalidField(
                "expected ConnectAck",
            ))),
        }
    }
}
