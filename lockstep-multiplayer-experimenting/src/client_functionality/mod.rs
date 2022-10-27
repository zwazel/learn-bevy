use std::borrow::BorrowMut;
use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::asset::{Assets, AssetServer, Handle};
use bevy::input::Input;
use bevy::math::Vec2;
use bevy::prelude::*;
use rand::Rng;
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient};

use crate::{client_connection_config, ClientChannel, ClientLobby, commands, NetworkMapping, Player, PlayerCommand, PlayerId, PlayerInfo, PROTOCOL_ID, ServerChannel, ServerLobby, ServerMarker, ServerMessages, ServerTick, Tick};
use crate::asset_handling::TargetAssets;
use crate::ClientMessages::ClientUpdateTick;
use crate::commands::{CommandQueue, ServerSyncedPlayerCommandsList, SyncedPlayerCommandsList};
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

pub fn handle_mouse_input(
    mut command_queue: ResMut<CommandQueue>,
    buttons: Res<Input<MouseButton>>,
) {
    if buttons.just_pressed(MouseButton::Right) {
        // Right button was pressed
        let command = PlayerCommand::SetTargetPosition(0.0,0.0);
        command_queue.add_command(command);
    }

    if buttons.just_pressed(MouseButton::Left) {
        // Left button was pressed
        let command = PlayerCommand::Test("Left button was pressed".to_string());
        command_queue.add_command(command);
    }
}


pub fn client_update_system(
    mut bevy_commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut most_recent_tick: ResMut<Tick>,
    mut most_recent_server_tick: ResMut<ServerTick>,
    is_server: Option<Res<ServerMarker>>,
    mut synced_commands: ResMut<SyncedPlayerCommandsList>,
    mut to_sync_commands: ResMut<CommandQueue>,
    target_assets: Res<TargetAssets>
) {
    let client_id = client.client_id();

    while let Some(message) = client.receive_message(ServerChannel::ServerMessages.id()) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate { player, entity } => {
                let is_player = client_id == player.id.0;

                let client_entity = bevy_commands
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
                    bevy_commands.entity(client_entity).despawn();
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
            UpdateTick { target_tick, commands } => {
                let username = lobby.get_username(PlayerId(client_id)).unwrap();
                most_recent_server_tick.0.0 = target_tick.0;

                let is_server = is_server.is_some();

                synced_commands.0.insert(target_tick, commands.clone());

                for (player_id, commands_list_of_player) in commands.0.0 {
                    let is_player = player_id.0 == client_id;
                    let command_username = lobby.get_username(player_id);
                    if let Some(command_username) = command_username {
                        for command in commands_list_of_player {
                            match command {
                                PlayerCommand::Test(text) => {
                                    if is_player {
                                        println!("I said '{}' in tick {}", text, target_tick.0);
                                    } else {
                                        println!("{} said '{}' in tick {}", command_username, text, target_tick.0);
                                    }
                                }
                                PlayerCommand::SetTargetPosition(x, y) => {
                                    bevy_commands
                                        .spawn_bundle(SpriteBundle {
                                            texture: if is_player {
                                                target_assets.friendly_target.clone()
                                            } else {
                                                target_assets.enemy_target.clone()
                                            },
                                            transform: Transform::from_xyz(x, y, 0.0),
                                            ..Default::default()
                                        });
                                }
                            }
                        }
                    } else {
                        println!("Unknown player sent a command!");
                    }
                }

                most_recent_tick.0 = most_recent_server_tick.0.0;

                let message = bincode::serialize(&ClientUpdateTick {
                    current_tick: *most_recent_tick,
                    commands: to_sync_commands.clone().0,
                }).unwrap();

                to_sync_commands.reset();

                // if !is_server {
                //     // wait a random amount between 0 and 2 seconds if it isnt the server
                //     let chance_to_wait = rand::thread_rng().gen_range(0..=100);
                //
                //     if chance_to_wait < 5 {
                //         let wait_time = rand::thread_rng().gen_range(0..=1000);
                //         println!("Client {} waiting {} ms before sending tick", username, wait_time);
                //         std::thread::sleep(std::time::Duration::from_millis(wait_time));
                //     }
                // }

                client.send_message(ClientChannel::ClientTick.id(), message);
            }
            _ => {
                panic!("Unexpected message on ServerTick channel");
            }
        }
    }
}