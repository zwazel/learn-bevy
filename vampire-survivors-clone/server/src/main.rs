use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;

use log::trace;
use renet::{RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig};

use store::{HOST, PORT};

pub const PROTOCOL_ID: u64 = 6969;

fn main() {
    println!("{}", HOST);
    println!("{}", PORT);
    let address = format!("{}:{}", HOST, PORT);
    println!("{}", address);

    let server_addr: SocketAddr = format!("{}:{}", HOST, PORT)
        .parse()
        .unwrap();

    let mut server: RenetServer = RenetServer::new(
        // Pass the current time to renet, so it can use it to order messages
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        // Pass a server configuration specifying that we want to allow only 2 clients to connect
        // and that we don't want to authenticate them. Everybody is welcome!
        ServerConfig::new(2, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure),
        // Pass the default connection configuration. This will create a reliable, unreliable and blocking channel.
        // We only actually need the reliable one, but we can just not use the other two.
        RenetConnectionConfig::default(),
        UdpSocket::bind(server_addr).unwrap(),
    )
        .unwrap();

    trace!("🕹  TicTacTussle server listening on {}", server_addr);
}