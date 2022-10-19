use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::prelude::{Commands, default, EventReader, ResMut};
use renet::{NETCODE_USER_DATA_BYTES, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};

use crate::{ClientChannel, ClientMessages, ClientTicks, Player, PlayerId, PROTOCOL_ID, server_connection_config, ServerLobby, ServerTick, Tick, Username};
use crate::commands::PlayerCommand;
use crate::ServerChannel::ServerMessages;
use crate::ServerMessages::{PlayerCreate, PlayerRemove};

/// Utility function for extracting a players name from renet user data
pub fn name_from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> String {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&user_data[0..8]);
    let mut len = u64::from_le_bytes(buffer) as usize;
    len = len.min(NETCODE_USER_DATA_BYTES - 8);
    let data = user_data[8..len + 8].to_vec();
    String::from_utf8(data).unwrap()
}

pub fn new_renet_server(amount_of_player: usize, host: &str, port: i32) -> RenetServer {
    let server_addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .unwrap();
    let socket = UdpSocket::bind(server_addr).unwrap();
    let connection_config = server_connection_config();
    let server_config = ServerConfig::new(amount_of_player, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure);
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    RenetServer::new(current_time, server_config, connection_config, socket).unwrap()
}

#[allow(clippy::too_many_arguments)]
pub fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut server: ResMut<RenetServer>,
    mut client_ticks: ResMut<ClientTicks>,
    mut server_ticks: ResMut<ServerTick>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, user_data) => {
                let username = name_from_user_data(&user_data);
                println!("Player {} connected.", username);

                let username = Username(username);

                let player_entity = commands
                    .spawn()
                    .insert(Player {
                        id: PlayerId(*id),
                        username: username.clone(),
                        ..default()
                    })
                    .id();

                lobby.0.insert(PlayerId(*id), Player {
                    id: PlayerId(*id),
                    username: username.clone(),
                    entity: Some(player_entity),
                });

                client_ticks.0.insert(PlayerId(*id), Tick::new());

                let message = bincode::serialize(&PlayerCreate {
                    player: Player {
                        id: PlayerId(*id),
                        username: username.clone(),
                        entity: Some(player_entity),
                    },
                    entity: player_entity,
                })
                    .unwrap();
                server.broadcast_message(ServerMessages.id(), message);
            }
            ServerEvent::ClientDisconnected(id) => {
                let username = lobby.0.get(&PlayerId(*id)).unwrap().username.clone();

                println!("Player {} disconnected.", username);
                client_ticks.0.remove(&PlayerId(*id));
                if let Some(player_entity) = lobby.0.remove(&PlayerId(*id)) {
                    commands.entity(player_entity.entity.unwrap()).despawn();
                }

                println!("Resetting Ticks");
                server_ticks.0 = Tick::new();

                client_ticks.0.iter_mut().for_each(|(_, tick)| {
                    tick.reset();
                });

                let message = bincode::serialize(&PlayerRemove { id: PlayerId(*id) }).unwrap();
                server.broadcast_message(ServerMessages.id(), message);
            }
        }
    }

    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Command.id()) {
            let command: PlayerCommand = bincode::deserialize(&message).unwrap();
            match command {
                PlayerCommand::Test { .. } => {}
            }
        }
        while let Some(message) = server.receive_message(client_id, ClientChannel::ClientTick.id()) {
            let username = lobby.get_username(PlayerId(client_id)).unwrap();
            let client_message: ClientMessages = bincode::deserialize(&message).unwrap();

            match client_message {
                ClientMessages::ClientUpdateTick { current_tick } => {
                    let client_tick = client_ticks.0.get_mut(&PlayerId(client_id)).unwrap();

                    println!("client {}: current server tick: {} -> client Tick processed: {}", username, client_tick.get(), current_tick.get());

                    client_tick.0 = current_tick.0;
                    println!("client {}: new tick: {}", username, client_tick.get());
                }
            }
        }
        // while let Some(message) = server.receive_message(client_id, ClientChannel::Input.id()) {
        //     let input: PlayerInput = bincode::deserialize(&message).unwrap();
        //     client_ticks.0.insert(PlayerId(client_id), input.most_recent_tick);
        //     if let Some(player_entity) = lobby.players.get(&client_id) {
        //         commands.entity(*player_entity).insert(input);
        //     }
        // }
    }
}