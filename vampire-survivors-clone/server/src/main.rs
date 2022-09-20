use std::{env, thread};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

use log::{info, trace, warn};
use renet::{NETCODE_USER_DATA_BYTES, RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};

use rand::prelude::*;
use store::{AMOUNT_PLAYERS, EndGameReason, HOST, PORT, PROTOCOL_ID, Position};

/// Utility function for extracting a players name from renet user data
fn name_from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> String {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&user_data[0..8]);
    let mut len = u64::from_le_bytes(buffer) as usize;
    len = len.min(NETCODE_USER_DATA_BYTES - 8);
    let data = user_data[8..len + 8].to_vec();
    String::from_utf8(data).unwrap()
}

fn translate_host(host: &str) -> &str {
    let host = match host {
        "localhost" => "127.0.0.1",
        _ => host,
    };
    host
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut port = PORT;
    let mut host = HOST;
    let mut amount_of_players = AMOUNT_PLAYERS;
    match args.len() {
        1 => {
            // no args
            println!("Default settings as no args passed, PLAYERS: {}, PORT: {}", amount_of_players, PORT);
        }
        2 => {
            amount_of_players = args[1].parse().unwrap();
            println!("Amount of players set to: {}", amount_of_players);
        }
        3 => {
            port = args[1].parse().unwrap();
            amount_of_players = args[2].parse().unwrap();
            println!("Amount of players has been set to: {}, Port has been set to: {}", amount_of_players, port);
        }
        _ => {
            println!("Too many args passed, please pass only 2 args, the amount of players and the port\n\
            using the default settings, PLAYERS: {}, PORT: {}", amount_of_players, PORT);
        }
    };

    let server_addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .unwrap();

    let mut server: RenetServer = RenetServer::new(
        // Pass the current time to renet, so it can use it to order messages
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        // Pass a server configuration specifying that we want to allow only 2 clients to connect
        // and that we don't want to authenticate them. Everybody is welcome!
        ServerConfig::new(amount_of_players, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure),
        // Pass the default connection configuration. This will create a reliable, unreliable and blocking channel.
        // We only actually need the reliable one, but we can just not use the other two.
        RenetConnectionConfig::default(),
        UdpSocket::bind(server_addr).unwrap(),
    )
        .unwrap();

    trace!("🕹  TicTacTussle server listening on {}", server_addr);

    let mut game_state = store::GameState::default();
    let mut last_updated = Instant::now();

    loop {
        // Update server time
        let now = Instant::now();
        server.update(now - last_updated).unwrap();
        last_updated = now;

        // Receive connection events from clients
        while let Some(event) = server.get_event() {
            match event {
                ServerEvent::ClientConnected(id, user_data) => {
                    // random position for new player
                    let mut rng = thread_rng();
                    let x = rng.gen_range(10..51);
                    let y = rng.gen_range(10..51);
                    let x = f64::from(x);
                    let y = f64::from(y);

                    // Tell the recently joined player about the other player
                    for (player_id, player) in game_state.players.iter() {
                        let event = store::GameEvent::PlayerJoined {
                            player_id: *player_id,
                            name: player.name.clone(),
                            pos: player.pos,
                        };
                        server.send_message(id, 0, bincode::serialize(&event).unwrap());
                    }

                    // Add the new player to the game
                    let event = store::GameEvent::PlayerJoined {
                        player_id: id,
                        name: name_from_user_data(&user_data),
                        pos: Position { x, y },
                    };
                    game_state.consume(&event);

                    // Tell all players that a new player has joined
                    server.broadcast_message(0, bincode::serialize(&event).unwrap());

                    info!("Client {} connected.", id);
                    // once two players have joined, start it
                    if game_state.players.len() == 2 {
                        let event = store::GameEvent::BeginGame;
                        game_state.consume(&event);
                        server.broadcast_message(0, bincode::serialize(&event).unwrap());
                        trace!("The game gas begun");
                    }
                }
                ServerEvent::ClientDisconnected(id) => {
                    // First consume a disconnect event
                    let event = store::GameEvent::PlayerDisconnected { player_id: id };
                    game_state.consume(&event);
                    server.broadcast_message(0, bincode::serialize(&event).unwrap());

                    // NOTE: Since we don't authenticate users we can't do any reconnection attempts.
                    // We simply have no way to know if the next user is the same as the one that disconnected.
                }
            }
        }

        // Receive GameEvents from clients. Broadcast valid events.
        for client_id in server.clients_id().into_iter() {
            while let Some(message) = server.receive_message(client_id, 0) {
                if let Ok(event) = bincode::deserialize::<store::GameEvent>(&message) {
                    if game_state.validate(&event) {
                        game_state.consume(&event);
                        trace!("Player {} sent:\n\t{:#?}", client_id, event);
                        server.broadcast_message(0, bincode::serialize(&event).unwrap());
                    } else {
                        warn!("Player {} sent invalid event:\n\t{:#?}", client_id, event);
                    }
                }
            }
        }

        server.send_packets().unwrap();
        thread::sleep(Duration::from_millis(50));
    }
}