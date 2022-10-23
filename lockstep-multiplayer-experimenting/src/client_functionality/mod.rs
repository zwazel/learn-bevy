use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::asset::{Assets, AssetServer};
use bevy::math::Vec2;
use bevy::prelude::{Commands, default, Res, ResMut, SpriteSheetBundle, TextureAtlas, TextureAtlasSprite, Transform};
use rand::Rng;
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient};

use crate::{client_connection_config, ClientChannel, ClientLobby, NetworkMapping, Player, PlayerCommand, PlayerId, PlayerInfo, PROTOCOL_ID, ServerChannel, ServerLobby, ServerMarker, ServerMessages, ServerTick, Tick};
use crate::ClientMessages::ClientUpdateTick;
use crate::ServerMessages::UpdateTick;

pub fn new_renet_client(username: &String, host: &str, port: i32) -> RenetClient {
    let server_addr = format!("{}:{}", host, port).parse().unwrap();
    let socket = UdpSocket::bind(format!("0.0.0.0:0")).unwrap();
    let connection_config = client_connection_config();
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;

    // Place username in user data
    let mut user_data = [0u8; NETCODE_USER_DATA_BYTES];
    if username.len() > NETCODE_USER_DATA_BYTES - 8 {
        panic!("Username is too big");
    }
    user_data[0..8].copy_from_slice(&(username.len() as u64).to_le_bytes());
    user_data[8..username.len() + 8].copy_from_slice(username.as_bytes());

    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: Some(user_data),
    };

    RenetClient::new(current_time, socket, client_id, connection_config, authentication).unwrap()
}

pub fn client_update_system(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut most_recent_tick: ResMut<Tick>,
    mut most_recent_server_tick: ResMut<ServerTick>,
    is_server: Option<Res<ServerMarker>>,
) {
    let client_id = client.client_id();

    while let Some(message) = client.receive_message(ServerChannel::ServerMessages.id()) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate { player, entity } => {
                let is_player = client_id == player.id.0;

                let client_entity = commands
                    .spawn()
                    .insert(Player {
                        id: player.id,
                        username: player.username.clone(),
                        entity: None,
                    })
                    .id();

                if is_player {
                    // client_entity.insert(ControlledPlayer);
                    println!("You're now connected to the server!")
                } else {
                    println!("Player {} connected to the server.", player.username);
                }

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity,
                    username: player.username.clone(),
                };
                lobby.0.insert(player.id, player_info);
                network_mapping.0.insert(entity, client_entity);
            }
            ServerMessages::PlayerRemove { id } => {
                let is_player = client_id == id.0;

                let username = lobby.get_username(id).unwrap();

                if is_player {
                    println!("You've been disconnected from the server!");
                } else {
                    println!("Player {} disconnected from the server.", username);
                }
                if let Some(PlayerInfo {
                                server_entity,
                                client_entity,
                                ..
                            }) = lobby.0.remove(&id)
                {
                    commands.entity(client_entity).despawn();
                    network_mapping.0.remove(&server_entity);
                }

                most_recent_server_tick.reset();
                most_recent_tick.reset();
            }
            _ => {
                panic!("Unexpected message on ServerMessages channel!")
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::ServerTick.id()) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            UpdateTick { target_tick } => {
                let username = lobby.get_username(PlayerId(client_id)).unwrap();
                most_recent_server_tick.0.0 = target_tick.0;

                let is_server = is_server.is_some();

                if !is_server {
                    println!("Client {} got server Tick to process: {}, was on tick: {}", username, most_recent_server_tick.get(), most_recent_tick.get());
                }

                most_recent_tick.0 = most_recent_server_tick.0.0;

                if !is_server {
                    println!("Client {} processed Tick, most recent tick now: {}", username, most_recent_tick.get());
                }

                let mut commands: Vec<PlayerCommand> = Vec::new();

                let chance_to_add_command = rand::thread_rng().gen_range(0..=100);
                if chance_to_add_command < 50 {
                    let command = PlayerCommand::Test(username.clone());
                    commands.push(command);
                }

                let message = bincode::serialize(&ClientUpdateTick {
                    current_tick: *most_recent_tick,
                    commands,
                }).unwrap();

                if !is_server {
                    // wait a random amount between 0 and 2 seconds if it isnt the server
                    let chance_to_wait = rand::thread_rng().gen_range(0..=100);

                    if chance_to_wait < 20 {
                        let wait_time = rand::thread_rng().gen_range(0..=2000);
                        println!("Client {} waiting {} ms before sending tick", username, wait_time);
                        std::thread::sleep(std::time::Duration::from_millis(wait_time));
                    }
                }

                client.send_message(ClientChannel::ClientTick.id(), message);
            }
            _ => {
                panic!("Unexpected message on ServerTick channel");
            }
        }
    }
}