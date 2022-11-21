use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::prelude::{Commands, default, EventReader, ResMut};
use renet::{NETCODE_USER_DATA_BYTES, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};

use crate::{ClientChannel, ClientMessages, ClientTicks, CurrentServerTick, Player, PlayerId, PROTOCOL_ID, server_connection_config, ServerChannel, ServerLobby, Tick, Username};
use crate::commands::{MyDateTime, PlayerCommandsList, ServerSyncedPlayerCommandsList, SyncedPlayerCommand};
use crate::ServerChannel::ServerMessages;
use crate::ServerMessages::{PlayerCreate, PlayerRemove, UpdateTick};

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

pub fn fixed_time_step_server(
    mut server_tick: ResMut<CurrentServerTick>,
    mut synced_commands: ResMut<ServerSyncedPlayerCommandsList>,
    mut server: Option<ResMut<RenetServer>>,
) {
    if let Some(server) = server.as_mut() { // we're server
        let server_tick = server_tick.as_mut();

        let commands = synced_commands.0.0.get(&Tick(server_tick.get()));

        server_tick.increment();

        let message = bincode::serialize(&UpdateTick {
            target_tick: server_tick.0,
            commands: {
                if let Some(commands) = commands {
                    commands.clone()
                } else {
                    SyncedPlayerCommand::default()
                }
            },
        }).unwrap();

        synced_commands.0.0.insert(server_tick.0, SyncedPlayerCommand(PlayerCommandsList::default(), MyDateTime::now()));

        server.broadcast_message(ServerChannel::ServerTick.id(), message);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut server: ResMut<RenetServer>,
    mut client_ticks: ResMut<ClientTicks>,
    mut server_ticks: ResMut<CurrentServerTick>,
    mut synced_commands: ResMut<ServerSyncedPlayerCommandsList>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, user_data) => {
                let username = name_from_user_data(&user_data);
                println!("Player {} connected.", username);

                let username = Username(username);

                for (_, player) in lobby.0.iter() {
                    let message = bincode::serialize(&PlayerCreate {
                        player: player.clone(),
                        entity: player.entity.unwrap(),
                    })
                        .unwrap();
                    server.send_message(*id, ServerMessages.id(), message);
                }

                let player_entity = commands
                    .spawn((
                        Player {
                            id: PlayerId(*id),
                            username: username.clone(),
                            ..default()
                        },
                    ))
                    .id();

                lobby.0.insert(PlayerId(*id), Player {
                    id: PlayerId(*id),
                    username: username.clone(),
                    entity: Some(player_entity),
                    movement: None,
                });

                client_ticks.0.insert(PlayerId(*id), Tick::new());

                let message = bincode::serialize(&PlayerCreate {
                    player: Player {
                        id: PlayerId(*id),
                        username: username.clone(),
                        entity: Some(player_entity),
                        movement: None
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
        while let Some(message) = server.receive_message(client_id, ClientChannel::ClientTick.id()) {
            let client_message: ClientMessages = bincode::deserialize(&message).unwrap();

            match client_message {
                ClientMessages::ClientUpdateTick { current_tick, commands } => {
                    let client_tick = client_ticks.0.get_mut(&PlayerId(client_id)).unwrap();

                    client_tick.0 = current_tick.0;
                    synced_commands.add_command(*client_tick, PlayerId(client_id), commands);
                }
            }
        }
    }
}
