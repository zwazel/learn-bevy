#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::app::AppExit;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::camera::RenderTarget::Window;
use bevy::window::{WindowClosed, WindowCloseRequested, WindowPlugin, WindowSettings};
use bevy_egui::{EguiContext, EguiPlugin};
use bevy_renet::{RenetClientPlugin, run_if_client_connected};
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetConnectionConfig, RenetError};
use renet_visualizer::{RenetClientVisualizer, RenetVisualizerStyle};
use smooth_bevy_cameras::{LookTransform, LookTransformBundle, Smoother};

use vampire_surviors_clone::{client_connection_config, ClientChannel, NetworkFrame, PlayerCommand, PlayerInput, PORT, PROTOCOL_ID, Ray3d, ServerChannel, ServerMessages, translate_host, translate_port};

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    let mut username = format!("Player_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let mut host = "127.0.0.1";
    let mut port = PORT;
    match args.len() {
        2 => {
            username = args[1].clone();
            println!("Username set to: {}", username);
        }
        3 => {
            username = args[1].clone();
            host = translate_host(&args[2], "");
            println!("Host has been set to: {}, Username has been set to: {}", host, username);
        }
        4 => {
            username = args[1].clone();
            host = translate_host(&args[2], "");
            port = translate_port(&args[3]);
            println!("Port has been set to: {}, Host has been set to: {}, Username has been set to: {}", port, host, username);
        }
        _ => {
            println!("Usage: client [username] [host] [port]");
            println!("Default values: username: {}, host: {}, port: {}", username, host, port);
        }
    }

    App::new()
        .insert_resource(WindowDescriptor {
            title: format!("Vampire Survivors Clone <{}>", username),
            width: 480.0,
            height: 540.0,
            ..default()
        })
        .insert_resource(WindowSettings {
            ..default()
        })
        .insert_resource(ClearColor(Color::hex("282828").unwrap()))
        .insert_resource(new_renet_client(&username, host, port))
        .insert_resource(PlayerHandles::default())
        .insert_resource(ClientLobby::default())
        .insert_resource(PlayerInput::default())
        .insert_resource(GameState::default())
        .insert_resource(MostRecentTick(None))
        .insert_resource(NetworkMapping::default())

        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(RenetClientPlugin)
        .add_plugin(EguiPlugin)

        .add_system(handle_renet_error)
        .add_system(player_input)
        .add_system(camera_follow)
        .add_system(client_send_input.with_run_criteria(run_if_client_connected))
        .add_system(client_send_player_commands.with_run_criteria(run_if_client_connected))
        .add_system(client_sync_players.with_run_criteria(run_if_client_connected))

        .insert_resource(RenetClientVisualizer::<200>::new(RenetVisualizerStyle::default()))
        .add_system(update_visulizer_system)

        .add_event::<PlayerCommand>()

        .add_startup_system(setup_camera)
        .add_system(panic_on_error_system)

        .add_system_to_stage(CoreStage::Last, disconnect)

        .run();
}

#[derive(Component)]
struct ControlledPlayer;

#[derive(Default)]
struct NetworkMapping(HashMap<Entity, Entity>);

#[derive(Debug)]
struct PlayerInfo {
    client_entity: Entity,
    server_entity: Entity,
}

#[derive(Debug, Default)]
struct ClientLobby {
    players: HashMap<u64, PlayerInfo>,
}

#[derive(Debug)]
struct MostRecentTick(Option<u32>);

////////// RENET NETWORKING //////////
fn new_renet_client(username: &String, host: &str, port: i32) -> RenetClient {
    let server_addr = format!("{}:{}", host, port).parse()?;
    let socket = UdpSocket::bind(format!("0.0.0.0:0"))?;
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

//noinspection RsTypeCheck
fn update_visulizer_system(
    mut egui_context: ResMut<EguiContext>,
    mut visualizer: ResMut<RenetClientVisualizer<200>>,
    client: Res<RenetClient>,
    mut show_visualizer: Local<bool>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    visualizer.add_network_info(client.network_info());
    if keyboard_input.just_pressed(KeyCode::F1) {
        *show_visualizer = !*show_visualizer;
    }
    if *show_visualizer {
        visualizer.show_window(egui_context.ctx_mut());
    }
}

fn player_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut player_input: ResMut<PlayerInput>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut player_commands: EventWriter<PlayerCommand>,
    most_recent_tick: Res<MostRecentTick>,
    cursor_moved_events: Res<Events<CursorMoved>>,
) {
    player_input.left = keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left);
    player_input.right = keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right);
    player_input.up = keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up);
    player_input.down = keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down);
    player_input.most_recent_tick = most_recent_tick.0;

    if mouse_button_input.just_pressed(MouseButton::Left) {
        player_commands.send(PlayerCommand::BasicAttack {
            cast_at: cursor_moved_events.iter().last().unwrap().position,
        });
    }
}

fn client_send_input(player_input: Res<PlayerInput>, mut client: ResMut<RenetClient>) {
    let input_message = bincode::serialize(&*player_input).unwrap();

    client.send_message(ClientChannel::Input.id(), input_message);
}

fn client_send_player_commands(mut player_commands: EventReader<PlayerCommand>, mut client: ResMut<RenetClient>) {
    for command in player_commands.iter() {
        let command_message = bincode::serialize(command).unwrap();
        client.send_message(ClientChannel::Command.id(), command_message);
    }
}

fn client_sync_players(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut most_recent_tick: ResMut<MostRecentTick>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let client_id = client.client_id();

    let texture_handle_self = asset_server.load("sprites/bob.png");
    let texture_handle_others = asset_server.load("sprites/fritz.png");
    let texture_handle_bullet = asset_server.load("sprites/bullet.png");
    let texture_atlas_self = TextureAtlas::from_grid(texture_handle_self, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_others = TextureAtlas::from_grid(texture_handle_others, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_bullet = TextureAtlas::from_grid(texture_handle_bullet, Vec2::new(16.0, 16.0), 1, 1);
    let texture_atlas_handle_self = texture_atlases.add(texture_atlas_self);
    let texture_atlas_handle_others = texture_atlases.add(texture_atlas_others);
    let texture_atlas_handle_bullet = texture_atlases.add(texture_atlas_bullet);

    while let Some(message) = client.receive_message(ServerChannel::ServerMessages.id()) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate { id, translation, entity } => {
                println!("Player {} connected.", id);

                let is_player = client_id == client.client_id();
                let texture_atlas_handle = if is_player {
                    texture_atlas_handle_self.clone()
                } else {
                    texture_atlas_handle_others.clone()
                };

                let mut client_entity = commands
                    .spawn_bundle(SpriteSheetBundle {
                        texture_atlas: texture_atlas_handle.clone(),
                        sprite: TextureAtlasSprite::new(0),
                        transform: Transform::from_xyz(translation[0], translation[1], 0.0),
                        ..Default::default()
                    });

                if is_player {
                    client_entity.insert(ControlledPlayer);
                }

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity: client_entity.id(),
                };
                lobby.players.insert(id, player_info);
                network_mapping.0.insert(entity, client_entity.id());
            }
            ServerMessages::PlayerRemove { id } => {
                println!("Player {} disconnected.", id);
                if let Some(PlayerInfo {
                                server_entity,
                                client_entity,
                            }) = lobby.players.remove(&id)
                {
                    commands.entity(client_entity).despawn();
                    network_mapping.0.remove(&server_entity);
                }
            }
            ServerMessages::SpawnProjectile { entity, translation } => {
                let projectile_entity = commands
                    .spawn_bundle(SpriteSheetBundle {
                        texture_atlas: texture_atlas_handle_bullet.clone(),
                        sprite: TextureAtlasSprite::new(0),
                        transform: Transform::from_xyz(translation[0], translation[1], 0.0),
                        ..Default::default()
                    });
                network_mapping.0.insert(entity, projectile_entity.id());
            }
            ServerMessages::DespawnProjectile { entity } => {
                if let Some(entity) = network_mapping.0.remove(&entity) {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::NetworkFrame.id()) {
        let frame: NetworkFrame = bincode::deserialize(&message).unwrap();
        match most_recent_tick.0 {
            None => most_recent_tick.0 = Some(frame.tick),
            Some(tick) if tick < frame.tick => most_recent_tick.0 = Some(frame.tick),
            _ => continue,
        }

        for i in 0..frame.entities.entities.len() {
            if let Some(entity) = network_mapping.0.get(&frame.entities.entities[i]) {
                let translation = frame.entities.translations[i].into();
                let transform = Transform {
                    translation,
                    ..Default::default()
                };
                commands.entity(*entity).insert(transform);
            }
        }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle {
        ..default()
    });
}

fn camera_follow(
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<ControlledPlayer>)>,
    player_query: Query<&Transform, With<ControlledPlayer>>,
) {
    let mut cam_transform = camera_query.single_mut();
    if let Ok(player_transform) = player_query.get_single() {
        cam_transform.translation = player_transform.translation;
    }
}

fn disconnect(
    mut events: EventReader<AppExit>,
    mut client: ResMut<RenetClient>,
) {
    if let Some(_) = events.iter().next() {
        print!("Exiting...");
        client.disconnect();
        std::process::exit(0);
    }
}

// If any error is found we just panic
fn panic_on_error_system(mut renet_error: EventReader<RenetError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}