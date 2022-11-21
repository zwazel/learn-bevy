

use std::net::UdpSocket;

use std::time::SystemTime;


use bevy::input::Input;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::math::{Vec2, Vec3};
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;

use bevy::render::camera::RenderTarget;
use bevy::window::CursorGrabMode;
use bevy_egui::egui::{lerp};
use bevy_mod_picking::{PickableBundle};

use bevy_rapier3d::plugin::RapierContext;
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group, InteractionGroups, QueryFilter};
use nalgebra::ComplexField;

use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient};




use crate::*;
use crate::ClientMessages::ClientUpdateTick;
use crate::commands::{CommandQueue, SyncedPlayerCommandsList};
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

    RenetClient::new(current_time, socket, connection_config, authentication).unwrap()
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

pub fn create_new_units(
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mouse_input: Res<Input<MouseButton>>,
    rapier_context: Res<RapierContext>,
    mut commands_to_sync: ResMut<CommandQueue>,
    camera_movement: Res<CameraMovement>,
) {
    let (camera, camera_transform) = query_camera.single();

    if mouse_input.just_pressed(MouseButton::Left) {
        let hits = raycast_to_world(windows, camera, camera_transform, rapier_context, false, camera_movement);

        for hit in hits {
            let command = PlayerCommand::SpawnUnit(hit.1);
            commands_to_sync.add_command(command);
        }
    }
}

pub fn place_move_target(
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mouse_input: Res<Input<MouseButton>>,
    rapier_context: Res<RapierContext>,
    mut commands_to_sync: ResMut<CommandQueue>,
    camera_movement: Res<CameraMovement>,
) {
    let (camera, camera_transform) = query_camera.single();

    if mouse_input.just_pressed(MouseButton::Right) {
        let hits = raycast_to_world(windows, camera, camera_transform, rapier_context, false, camera_movement);

        for hit in hits {
            let command = PlayerCommand::SetTargetPosition(hit.1);
            commands_to_sync.add_command(command);
        }
    }
}

pub fn raycast_to_world(
    windows: Res<Windows>,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    rapier_context: Res<RapierContext>,
    return_multiple_hits: bool,
    camera_movement: Res<CameraMovement>,
) -> Vec<(Entity, Vec3, Vec3, f32)> {
    let window = if let RenderTarget::Window(id) = camera.target {
        windows.get(id).unwrap()
    } else {
        windows.get_primary().unwrap()
    };
    let ray;
    if let Some(cursor_position) = window.cursor_position() {
        ray = camera.viewport_to_world(camera_transform, cursor_position).unwrap();
    } else {
        ray = camera.viewport_to_world(camera_transform, camera_movement.last_mouse_position).unwrap();
    }
    let solid = true;
    let group = 0b0010;
    // println!("Group: {}", group);
    let groups = InteractionGroups::new(group.into(), group.into());
    let filter = QueryFilter::new().groups(groups);
    let max_toi = 1000.0;

    let mut hits: Vec<(Entity, Vec3, Vec3, f32)> = Vec::new();

    rapier_context.intersections_with_ray(
        ray.origin, ray.direction, max_toi, solid, filter,
        |entity, intersection| {
            // Callback called on each collider hit by the ray.
            let hit_point = intersection.point;
            let hit_normal = intersection.normal;
            let hit_toi = intersection.toi;

            hits.push((entity, hit_point, hit_normal, hit_toi));

            return_multiple_hits // Return `false` if we want to stop searching for other hits.
        });

    hits
}

pub fn move_camera(
    mut q_camera: Query<&mut Transform, (With<MainCamera>, Without<Camera>)>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut motion_evr: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    mut camera_movement: ResMut<CameraMovement>,
    mut camera_settings: ResMut<CameraSettings>,
    time: Res<Time>,
    mut windows: ResMut<Windows>,
) {
    let window = windows.get_primary_mut().unwrap();
    let mut camera_transform = q_camera.single_mut();
    let mut scroll_speed = camera_settings.scroll_speed;
    let mut rotation_speed = camera_settings.rotation_speed;

    if keyboard_input.pressed(KeyCode::LShift) {
        camera_settings.max_speed = DefaultSpeeds::Sprint.get();
        scroll_speed = camera_settings.scroll_sprint_speed;
        rotation_speed = camera_settings.rotation_sprint_speed;
    } else {
        camera_settings.max_speed = DefaultSpeeds::Normal.get();
    }

    let direction = Vec3::new(
        (keyboard_input.pressed(KeyCode::D) as i32 - keyboard_input.pressed(KeyCode::A) as i32) as f32,
        0.0,
        (keyboard_input.pressed(KeyCode::W) as i32 - keyboard_input.pressed(KeyCode::S) as i32) as f32,
    );

    if mouse_input.just_pressed(MouseButton::Middle) {
        window.set_cursor_grab_mode(CursorGrabMode::Confined);
        window.set_cursor_visibility(false);

        if let Some(cursor_position) = window.cursor_position() {
            camera_movement.last_mouse_position = cursor_position;
        }
    }

    if mouse_input.just_released(MouseButton::Middle) {
        window.set_cursor_grab_mode(CursorGrabMode::None);
        window.set_cursor_visibility(true);

        // set cursor position to the center of the screen
        window.set_cursor_position(camera_movement.last_mouse_position);

        camera_movement.last_mouse_position = Vec2::new(0.0, 0.0);
    }

    let current_yaw = camera_movement.mouse_yaw;
    let current_pitch = camera_movement.mouse_pitch;

    if mouse_input.pressed(MouseButton::Middle) {
        for event in motion_evr.iter() {
            // rotate camera, only left/right and up/down, no roll
            camera_movement.mouse_pitch -= event.delta.y * camera_settings.mouse_sensitivity * time.delta_seconds(); // up/down
            camera_movement.mouse_yaw -= event.delta.x * camera_settings.mouse_sensitivity * time.delta_seconds(); // left/right
        }
    } else {
        motion_evr.clear();
    }

    if keyboard_input.pressed(KeyCode::R) {
        camera_movement.mouse_yaw = 0.0;
        camera_movement.mouse_pitch = 0.0;
        camera_transform.rotation = Quat::from_rotation_y(0.0);
    }
    if keyboard_input.pressed(KeyCode::Q) {
        camera_movement.mouse_yaw += rotation_speed * time.delta_seconds(); // left/right
    }
    if keyboard_input.pressed(KeyCode::E) {
        camera_movement.mouse_yaw -= rotation_speed * time.delta_seconds(); // left/right
    }

    if camera_movement.mouse_yaw != current_yaw || camera_movement.mouse_pitch != current_pitch {
        // clamp pitch to prevent camera from flipping
        camera_movement.mouse_pitch = camera_movement.mouse_pitch.clamp(camera_settings.mouse_pitch_min_max.0, camera_settings.mouse_pitch_min_max.1);

        // keep yaw in 0..360 range
        camera_movement.mouse_yaw = camera_movement.mouse_yaw.rem_euclid(camera_settings.mouse_yaw_min_max.1);

        camera_transform.rotation = Quat::from_rotation_y(camera_movement.mouse_yaw.to_radians())
            * Quat::from_rotation_x(camera_movement.mouse_pitch.to_radians());
    }

    let mut forward = camera_transform.forward();
    let mut right = camera_transform.right();

    forward.y = 0.0;
    right.y = 0.0;
    forward = forward.normalize();
    right = right.normalize();

    let forward_relative_vertical_input = direction.z * forward;
    let right_relative_horizontal_input = direction.x * right;

    let mut camera_movement_direction = forward_relative_vertical_input + right_relative_horizontal_input;

    if camera_movement_direction.length() != 0.0 {
        camera_movement_direction = camera_movement_direction.normalize();
    };

    let mut scroll_direction = 0.0;
    for event in scroll_events.iter() {
        let increase = event.y * camera_settings.scroll_acceleration;
        scroll_direction += increase;
        camera_movement.target_camera_height += increase;
    }

    let mut spd = ComplexField::try_sqrt(camera_movement.velocity.x * camera_movement.velocity.x + camera_movement.velocity.z * camera_movement.velocity.z).unwrap();
    if camera_movement_direction.length() == 0.0 {
        // decelerate camera
        if spd <= camera_settings.deceleration {
            camera_movement.velocity = Vec3::ZERO;
        } else {
            camera_movement.velocity.x -= camera_movement.velocity.x / spd * camera_settings.deceleration;
            camera_movement.velocity.z -= camera_movement.velocity.z / spd * camera_settings.deceleration;
        }
    } else {
        if camera_movement.velocity.x * camera_movement_direction.x + camera_movement.velocity.z * camera_movement_direction.z < 0.0 {
            // skid
            if spd <= camera_settings.skid_deceleration {
                camera_movement.velocity = Vec3::ZERO;
            } else {
                camera_movement.velocity.x -= camera_movement.velocity.x / spd * camera_settings.skid_deceleration;
                camera_movement.velocity.z -= camera_movement.velocity.z / spd * camera_settings.skid_deceleration;
            }
        } else {
            // accelerate camera
            camera_movement.velocity.x += camera_movement_direction.x * camera_settings.acceleration;
            camera_movement.velocity.z += camera_movement_direction.z * camera_settings.acceleration;
            spd = ComplexField::try_sqrt(camera_movement.velocity.x * camera_movement.velocity.x + camera_movement.velocity.z * camera_movement.velocity.z).unwrap();
            if spd > camera_settings.max_speed.get().length() {
                camera_movement.velocity.x = camera_movement.velocity.x / spd * camera_settings.max_speed.get().x;
                camera_movement.velocity.z = camera_movement.velocity.z / spd * camera_settings.max_speed.get().z;
            }
        }
    }

    // move camera
    camera_transform.translation += camera_movement.velocity * time.delta_seconds();

    let target = camera_transform.translation.y + camera_movement.target_camera_height;
    // if distance between target and current height is greater than 0.1, move camera
    if (target - camera_transform.translation.y).abs() > camera_settings.scroll_error_tolerance {
        camera_transform.translation.y = lerp(camera_transform.translation.y..=target, scroll_speed * time.delta_seconds());
    }

    let scroll_spd = ComplexField::try_sqrt(camera_movement.target_camera_height * camera_movement.target_camera_height).unwrap();
    if scroll_direction == 0.0 {
        // decelerate camera
        if scroll_spd <= camera_settings.scroll_deceleration {
            camera_movement.target_camera_height = 0.0;
        } else {
            camera_movement.target_camera_height -= camera_movement.target_camera_height / scroll_spd * camera_settings.scroll_deceleration;
        }
    }
}

pub fn interpolate_movement_of_other_players(
    mut players: Query<(Entity, &mut Player, &mut Transform), Without<MainCamera>>,
    time: Res<Time>,
) {
    let _camera_settings = CameraSettings::default();

    for (_, mut player, mut entity_transform) in players.iter_mut() {
        let player: &mut Player = &mut player;
        let entity_transform: &mut Transform = &mut entity_transform;
        if let Some(player_movement) = player.movement {
            if player_movement.velocity.length() != 0.0 {
                entity_transform.translation += player_movement.velocity * time.delta_seconds();
            }
        }
    }
}

pub fn fixed_time_step_client(
    mut to_sync_commands: ResMut<CommandQueue>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut player_target_query: Query<Entity, (With<Target>, With<PlayerControlled>)>,
    mut other_target_query: Query<(Entity, &OtherPlayerControlled), With<Target>>,
    camera_movement: Res<CameraMovement>,
    mut q_camera: Query<&mut Transform, (With<MainCamera>, Without<Camera>)>,
    mut players: Query<(Entity, &mut Player, &mut Transform), Without<MainCamera>>,
    mut bevy_commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut most_recent_tick: ResMut<Tick>,
    most_recent_server_tick: ResMut<LocalServerTick>,
    synced_commands: ResMut<SyncedPlayerCommandsList>,
) {
    let client_id = client.client_id();
    let camera_transform = q_camera.single_mut();

    for (player_id, commands_list_of_player) in synced_commands.get_commands_for_tick(*most_recent_tick).0 {
        let is_player = player_id.0 == client_id;
        let command_username = lobby.get_username(player_id);
        if let Some(command_username) = command_username {
            for command in commands_list_of_player {
                match command {
                    PlayerCommand::Test(text) => {
                        if is_player {
                            println!("I said '{}' in tick {}", text, most_recent_server_tick.get());
                        } else {
                            println!("{} said '{}' in tick {}", command_username, text, most_recent_server_tick.get());
                        }
                    }
                    PlayerCommand::SetTargetPosition(vec3) => {
                        if is_player {
                            for player_target in player_target_query.iter_mut() {
                                bevy_commands.entity(player_target).despawn_descendants();
                            }
                        } else {
                            for (other_target, other_player_controlled) in other_target_query.iter_mut() {
                                if other_player_controlled.0 == player_id {
                                    bevy_commands.entity(other_target).despawn_descendants();
                                }
                            }
                        }

                        let target_entity = bevy_commands
                            .spawn(SpatialBundle {
                                transform: Transform::from_xyz(vec3.x, vec3.y + 0.5, vec3.z),
                                ..Default::default()
                            })
                            .insert(Target)
                            .id();

                        let collider = bevy_commands
                            .spawn(PbrBundle {
                                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                                material: if is_player {
                                    materials.add(Color::YELLOW.into())
                                } else {
                                    materials.add(Color::ORANGE_RED.into())
                                },
                                ..default()
                            })
                            .with_children(|parent| {
                                parent
                                    .spawn(Collider::cuboid(0.25, 0.25, 0.25))
                                    .insert(CollisionGroups::new(Group::GROUP_3, Group::GROUP_3))
                                    .insert(TransformBundle {
                                        ..Default::default()
                                    });
                            })
                            .id();

                        bevy_commands.entity(target_entity).push_children(&[collider]);

                        if is_player {
                            bevy_commands.entity(target_entity).insert(PlayerControlled);
                        } else {
                            bevy_commands.entity(target_entity).insert(OtherPlayerControlled(player_id));
                        }
                    }
                    PlayerCommand::SpawnUnit(vec3) => {
                        let unit_entity = bevy_commands
                            .spawn(SpatialBundle {
                                transform: Transform::from_xyz(vec3.x, vec3.y + 0.5, vec3.z),
                                ..Default::default()
                            })
                            .insert(Unit)
                            .id();

                        let collider = bevy_commands
                            .spawn(PbrBundle {
                                mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                                material: if is_player {
                                    materials.add(Color::rgb(0.8, 0.7, 0.6).into())
                                } else {
                                    materials.add(Color::rgb(0.6, 0.7, 0.8).into())
                                },
                                ..default()
                            })
                            .with_children(|parent| {
                                parent
                                    .spawn(Collider::cuboid(0.5, 0.5, 0.5))
                                    .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_1))
                                    .insert(TransformBundle {
                                        ..Default::default()
                                    });
                            })
                            .id();

                        if is_player {
                            bevy_commands.entity(unit_entity)
                                .insert(PlayerControlled);
                            bevy_commands.entity(collider)
                                .insert(PickableBundle::default());
                        } else {
                            bevy_commands.entity(unit_entity)
                                .insert(OtherPlayerControlled(player_id));
                        }

                        bevy_commands.entity(unit_entity).push_children(&[collider]);
                    }
                    PlayerCommand::UpdatePlayerPosition(movement, transform) => {
                        if let Some(_player_info) = lobby.0.get_mut(&player_id) {
                            for (_, mut player, mut entity_transform) in players.iter_mut() {
                                if player.id.0 == player_id.0 {
                                    entity_transform.translation = transform.translation;
                                    entity_transform.rotation = transform.rotation;
                                    entity_transform.scale = transform.scale;

                                    player.movement = Some(movement);

                                    break;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            println!("Unknown player sent a command!");
        }
    }
    to_sync_commands.add_command(PlayerCommand::UpdatePlayerPosition(
        camera_movement.clone(),
        SerializableTransform::from_transform(camera_transform.clone()),
    ));

    most_recent_tick.0 = most_recent_server_tick.0.0;

    let message = bincode::serialize(&ClientUpdateTick {
        current_tick: *most_recent_tick,
        commands: to_sync_commands.clone().0,
    }).unwrap();

    to_sync_commands.reset();
    client.send_message(ClientChannel::ClientTick.id(), message);
}

pub fn client_update_system(
    mut bevy_commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut most_recent_tick: ResMut<Tick>,
    mut most_recent_server_tick: ResMut<LocalServerTick>,
    mut synced_commands: ResMut<SyncedPlayerCommandsList>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let client_id = client.client_id();

    while let Some(message) = client.receive_message(ServerChannel::ServerMessages.id()) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate { player, entity } => {
                let is_player = client_id == player.id.0;

                let client_entity = bevy_commands
                    .spawn((
                        Player {
                            id: player.id,
                            username: player.username.clone(),
                            entity: None,
                            movement: player.movement,
                        },
                    ))
                    .id();

                if is_player {
                    // client_entity.insert(ControlledPlayer);
                    println!("You're now connected to the server!")
                } else {
                    println!("Player {} connected to the server.", player.username);

                    bevy_commands.entity(client_entity)
                        .insert(OtherPlayerControlled(player.id))
                        .insert(SpatialBundle {
                            transform: Transform::from_xyz(0.0, 2.5, 5.0),
                            ..default()
                        })
                        .with_children(|children| {
                            children
                                .spawn(SpotLightBundle {
                                    spot_light: SpotLight {
                                        range: 500.0,
                                        intensity: 1000.0,
                                        shadows_enabled: true,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                });
                            children
                                .spawn(PbrBundle {
                                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
                                    material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                                    transform: Transform::from_xyz(0.0, 0.0, -1.5),
                                    ..Default::default()
                                })
                                .insert(NotShadowCaster);
                            children
                                .spawn(PbrBundle {
                                    mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                                    material: materials.add(Color::YELLOW_GREEN.into()),
                                    ..default()
                                });
                        });
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
                    bevy_commands.entity(client_entity).despawn_descendants();
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
            }
            _ => {
                panic!("Unexpected message on ServerTick channel");
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct SerializableTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl SerializableTransform {
    pub fn from_transform(transform: Transform) -> Self {
        Self {
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }
}
