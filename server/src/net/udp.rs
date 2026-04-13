use std::io;

use tokio::net::UdpSocket;

pub const MAX_DATAGRAM_SIZE: usize = 1400;
const RECV_BUFFER_SIZE: usize = 2048;

pub async fn run_udp_server(port: u16) -> io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", port)).await?;
    println!("[udp] listening on {}", socket.local_addr()?);

    let mut buf = [0u8; RECV_BUFFER_SIZE];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;

        if len > MAX_DATAGRAM_SIZE {
            eprintln!("[udp] dropped oversized datagram ({len} bytes) from {addr}");
            continue;
        }

        println!("[udp] received {len} bytes from {addr}");
    }
}
