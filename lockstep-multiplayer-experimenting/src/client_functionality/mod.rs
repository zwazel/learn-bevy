use std::net::UdpSocket;
use std::ops::Mul;
use std::time::SystemTime;

use bevy::ecs::query::OrFetch;
use bevy::input::Input;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use rand::Rng;
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient};

use crate::*;
use crate::asset_handling::{TargetAssets, UnitAssets};
use crate::ClientMessages::ClientUpdateTick;
use crate::commands::{CommandQueue, ServerSyncedPlayerCommandsList, SyncedPlayerCommandsList};
use crate::entities::{MoveTarget, OtherPlayerControlled, PlayerControlled, Target, Unit};
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
    windows: Res<Windows>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, camera_transform) = q_camera.single();
    let camera: &Camera = camera;
    let camera_transform: &GlobalTransform = camera_transform;

    // get the window that the camera is displaying to (or the primary window)
    let wnd = if let RenderTarget::Window(id) = camera.target {
        windows.get(id).unwrap()
    } else {
        windows.get_primary().unwrap()
    };

    let mut world_cursor_pos: Vec2 = Vec2::ZERO;
    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = wnd.cursor_position() {
        // get the size of the window
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        world_cursor_pos = world_pos.truncate();

        // round x and y to .2 precision
        world_cursor_pos.x = (world_cursor_pos.x * 100.0).round() * 0.01;
        world_cursor_pos.y = (world_cursor_pos.y * 100.0).round() * 0.01;
    }

    if buttons.just_pressed(MouseButton::Right) {
        // Right button was pressed
        let command = PlayerCommand::SetTargetPosition(world_cursor_pos.x, world_cursor_pos.y);
        command_queue.add_command(command);
    }

    if buttons.just_pressed(MouseButton::Left) {
        // Left button was pressed
        let command = PlayerCommand::SpawnUnit(world_cursor_pos.x, world_cursor_pos.y);
        command_queue.add_command(command);
    }
}

pub fn move_units(mut unit_query: Query<(&MoveTarget, &mut Transform), With<Unit>>, time: Res<Time>) {
    for (move_target, mut transform) in unit_query.iter_mut() {
        let move_target: &MoveTarget = move_target;

        let mut direction = Vec2 { x: move_target.0, y: move_target.1 } - transform.translation.truncate();
        let distance = direction.length();
        if distance > 0.1 {
            direction = direction.normalize();
            transform.translation.x += direction.x * 0.5 * time.delta_seconds();
            transform.translation.y += direction.y * 0.5 * time.delta_seconds();
        }
    }
}

pub fn move_camera(
    mut q_camera: Query<&mut Transform, With<MainCamera>>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut camera_transform = q_camera.single_mut();
    let move_speed = 2.0;
    let sprint_speed = 6.0;
    let mut speed = move_speed;

    let mut direction = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::W) {
        direction.z -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        direction.z += 1.0;
    }
    if keyboard_input.pressed(KeyCode::A) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) {
        direction.x += 1.0;
    }

    if keyboard_input.pressed(KeyCode::LShift) {
        speed = sprint_speed;
    }

    if direction.length() > 0.0 {
        direction = direction.normalize();
        camera_transform.translation.x += direction.x * speed * time.delta_seconds();
        camera_transform.translation.z += direction.z * speed * time.delta_seconds();
    }
}

pub fn client_update_system(
    mut bevy_commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut most_recent_tick: ResMut<Tick>,
    mut most_recent_server_tick: ResMut<ServerTick>,
    mut synced_commands: ResMut<SyncedPlayerCommandsList>,
    mut to_sync_commands: ResMut<CommandQueue>,
    target_assets: Res<TargetAssets>,
    unit_assets: Res<UnitAssets>,
    mut unit_query: Query<(Entity, Option<&MoveTarget>, Option<&PlayerControlled>, Option<&OtherPlayerControlled>), With<Unit>>,
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
                most_recent_server_tick.0.0 = target_tick.0;

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
                                    let target_entity = bevy_commands
                                        .spawn_bundle(SpriteBundle {
                                            texture: if is_player {
                                                target_assets.friendly_target.clone()
                                            } else {
                                                target_assets.enemy_target.clone()
                                            },
                                            transform: Transform::from_xyz(x, y, 0.0),
                                            ..Default::default()
                                        })
                                        .insert(Target(player_id))
                                        .id();
                                    if is_player {
                                        bevy_commands.entity(target_entity).insert(PlayerControlled);
                                    } else {
                                        bevy_commands.entity(target_entity).insert(OtherPlayerControlled(player_id));
                                    }

                                    for (entity, optional_move_target, optional_player_controlled, optional_other_controlled) in unit_query.iter_mut() {
                                        let mut add_command = false;

                                        if let Some(PlayerControlled) = optional_player_controlled {
                                            if is_player {
                                                add_command = true;
                                            }
                                        } else if let Some(OtherPlayerControlled(other_player_id)) = optional_other_controlled {
                                            if other_player_id.0 == player_id.0 {
                                                add_command = true;
                                            }
                                        }

                                        if add_command {
                                            if let Some(_) = optional_move_target {
                                                bevy_commands.entity(entity).remove::<MoveTarget>();
                                            }

                                            bevy_commands.entity(entity).insert(MoveTarget(x, y));
                                        }
                                    }
                                }
                                PlayerCommand::SpawnUnit(x, y) => {
                                    let unit_entity = bevy_commands
                                        .spawn_bundle(SpriteBundle {
                                            texture: if is_player {
                                                unit_assets.friendly.clone()
                                            } else {
                                                unit_assets.enemy.clone()
                                            },
                                            transform: Transform::from_xyz(x, y, 0.0),
                                            ..Default::default()
                                        })
                                        .insert(Unit)
                                        .id();

                                    if is_player {
                                        bevy_commands.entity(unit_entity).insert(PlayerControlled);
                                    } else {
                                        bevy_commands.entity(unit_entity).insert(OtherPlayerControlled(player_id));
                                    }
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
                client.send_message(ClientChannel::ClientTick.id(), message);
            }
            _ => {
                panic!("Unexpected message on ServerTick channel");
            }
        }
    }
}