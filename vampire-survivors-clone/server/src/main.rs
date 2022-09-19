use std::{env, thread};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

use log::{info, trace, warn};
use renet::{NETCODE_USER_DATA_BYTES, RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};

use store::{EndGameReason, HOST, PORT, PROTOCOL_ID};

/// Utility function for extracting a players name from renet user data
fn name_from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> String {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&user_data[0..8]);
    let mut len = u64::from_le_bytes(buffer) as usize;
    len = len.min(NETCODE_USER_DATA_BYTES - 8);
    let data = user_data[8..len + 8].to_vec();
    String::from_utf8(data).unwrap()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut port = PORT;
    let mut host = HOST;
    let mut amount_of_players = 2;
    match args.len() {
        1 => {
            // no args
            println!("Default settings as no args passed, PORT: {}, HOST: {}, PLAYERS: {}", PORT, HOST, amount_of_players);
        }
        2 => {
            println!("Port has been set to: {}", args[1]);
            port = args[1].parse().unwrap();
        }
        3 => {
            println!("Port has been set to: {}, Host has been set to: {}", args[1], args[2]);
            port = args[1].parse().unwrap();
            host = &*args[2];
        }
        4 => {
            println!("Port has been set to: {}, Host has been set to: {}, Amount of players has been set to: {}", args[1], args[2], args[3]);
            port = args[1].parse().unwrap();
            host = &*args[2];
            amount_of_players = args[3].parse().unwrap();
        }
        _ => {
            // more than one arg
            println!("Too many args passed, set to default, PORT: {}, HOST: {}, PLAYERS: {}", PORT, HOST, amount_of_players);
        }
    };

    let server_addr: SocketAddr = format!("{}:{}", HOST, port)
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
                    // Tell the recently joined player about the other player
                    for (player_id, player) in game_state.players.iter() {
                        let event = store::GameEvent::PlayerJoined {
                            player_id: *player_id,
                            name: player.name.clone(),
                        };
                        server.send_message(id, 0, bincode::serialize(&event).unwrap());
                    }

                    // Add the new player to the game
                    let event = store::GameEvent::PlayerJoined {
                        player_id: id,
                        name: name_from_user_data(&user_data),
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
                    info!("Client {} disconnected", id);

                    // Then end the game
                    let event = store::GameEvent::EndGame {
                        reason: EndGameReason::PlayerEndedTheGame { player_id: id },
                    };
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