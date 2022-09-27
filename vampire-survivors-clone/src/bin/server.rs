use std::{env, f32, thread};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

use bevy::app::{App, CoreStage};
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin};
use bevy_rapier3d::dynamics::{LockedAxes, RigidBody, Velocity};
use bevy_rapier3d::geometry::Collider;
use bevy_rapier3d::plugin::{NoUserData, RapierPhysicsPlugin};
use bevy_rapier3d::prelude::RapierDebugRenderPlugin;
use bevy_renet::RenetServerPlugin;
use log::{info, trace, warn};
use rand::prelude::*;
use renet::{NETCODE_USER_DATA_BYTES, RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};

use vampire_surviors_clone::{AMOUNT_PLAYERS, ClientChannel, NetworkFrame, Player, PlayerCommand, PlayerInput, PORT, Projectile, PROTOCOL_ID, server_connection_config, ServerChannel, ServerMessages, spawn_bullet, translate_host, translate_port};

/// Utility function for extracting a players name from renet user data
fn name_from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> String {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&user_data[0..8]);
    let mut len = u64::from_le_bytes(buffer) as usize;
    len = len.min(NETCODE_USER_DATA_BYTES - 8);
    let data = user_data[8..len + 8].to_vec();
    String::from_utf8(data).unwrap()
}

fn translate_amount_players(amount_players: &str) -> usize {
    amount_players.parse::<usize>().unwrap_or(AMOUNT_PLAYERS)
}

#[derive(Debug, Default)]
pub struct ServerLobby {
    pub players: HashMap<u64, Entity>,
}

#[derive(Debug, Default)]
struct NetworkTick(u32);

// Clients last received ticks
#[derive(Debug, Default)]
struct ClientTicks(HashMap<u64, Option<u32>>);

const PLAYER_MOVE_SPEED: f32 = 5.0;

fn new_renet_server(amount_of_player: usize, host: &str, port: i32) -> RenetServer {
    let server_addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .unwrap();
    let socket = UdpSocket::bind(server_addr).unwrap();
    let connection_config = server_connection_config();
    let server_config = ServerConfig::new(amount_of_player, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure);
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    RenetServer::new(current_time, server_config, connection_config, socket).unwrap()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut port = PORT;
    let mut host = "127.0.0.1";
    let mut amount_of_players = AMOUNT_PLAYERS;
    match args.len() {
        2 => {
            amount_of_players = translate_amount_players(&args[1]);
            println!("Amount of players set to: {}", amount_of_players);
        }
        3 => {
            port = translate_port(&args[2]);
            amount_of_players = translate_amount_players(&args[1]);
            println!("Amount of players has been set to: {}, Port has been set to: {}", amount_of_players, port);
        }
        4 => {
            host = translate_host(&args[3], "");
            port = translate_port(&args[2]);
            amount_of_players = translate_amount_players(&args[1]);
            println!("Amount of players has been set to: {}, Port has been set to: {}, Host has been set to: {}", amount_of_players, port, host);
        }
        _ => {
            println!("Usage: server [amount of players] [port] [host]");
            println!("Default values: amount of players: {}, port: {}, host: {}", AMOUNT_PLAYERS, PORT, host);
        }
    };

    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    app.add_plugin(RenetServerPlugin);
    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugin(RapierDebugRenderPlugin::default());
    app.add_plugin(FrameTimeDiagnosticsPlugin::default());
    app.add_plugin(LogDiagnosticsPlugin::default());
    app.add_plugin(EguiPlugin);

    app.insert_resource(ServerLobby::default());
    app.insert_resource(NetworkTick(0));
    app.insert_resource(ClientTicks::default());
    app.insert_resource(new_renet_server(amount_of_players, host, port));

    app.add_system(server_update_system);
    app.add_system(server_network_sync);
    app.add_system(move_players_system);
    app.add_system(update_projectiles_system);
    app.add_system_to_stage(CoreStage::PostUpdate, projectile_on_removal_system);

    app.run();
}

#[allow(clippy::too_many_arguments)]
fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut server: ResMut<RenetServer>,
    mut client_ticks: ResMut<ClientTicks>,
    players: Query<(Entity, &Player, &Transform)>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle_self = asset_server.load("sprites/bob.png");
    let texture_atlas_self = TextureAtlas::from_grid(texture_handle_self, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_handle_self = texture_atlases.add(texture_atlas_self);

    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _) => {
                println!("Player {} connected.", id);

                // Initialize other players for this new client
                for (entity, player, transform) in players.iter() {
                    let translation: [f32; 3] = transform.translation.into();
                    let message = bincode::serialize(&ServerMessages::PlayerCreate {
                        id: player.id,
                        entity,
                        translation: [translation[0], translation[1]],
                    })
                        .unwrap();
                    server.send_message(*id, ServerChannel::ServerMessages.id(), message);
                }

                // Spawn new player
                let transform = Transform::from_xyz(0.0, 0.51, 0.0);

                let player_entity = commands
                    .spawn_bundle(SpriteSheetBundle {
                        texture_atlas: texture_atlas_handle_self.clone(),
                        sprite: TextureAtlasSprite::new(0),
                        transform,
                        ..Default::default()
                    })
                    .insert(Player { id: *id })
                    .insert(PlayerInput::default())
                    .id();

                lobby.players.insert(*id, player_entity);

                let translation: [f32; 3] = transform.translation.into();
                let message = bincode::serialize(&ServerMessages::PlayerCreate {
                    id: *id,
                    entity: player_entity,
                    translation: [translation[0], translation[1]],
                })
                    .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages.id(), message);
            }
            ServerEvent::ClientDisconnected(id) => {
                println!("Player {} disconnected.", id);
                client_ticks.0.remove(id);
                if let Some(player_entity) = lobby.players.remove(id) {
                    commands.entity(player_entity).despawn();
                }

                let message = bincode::serialize(&ServerMessages::PlayerRemove { id: *id }).unwrap();
                server.broadcast_message(ServerChannel::ServerMessages.id(), message);
            }
        }
    }

    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Command.id()) {
            let command: PlayerCommand = bincode::deserialize(&message).unwrap();
            match command {
                PlayerCommand::BasicAttack { mut cast_at } => {
                    println!("Received basic attack from client {}: {:?}", client_id, cast_at);

                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        if let Ok((_, _, player_transform)) = players.get(*player_entity) {
                            cast_at[1] = player_transform.translation[1];

                            let direction = (cast_at - player_transform.translation).normalize_or_zero();
                            let mut translation = player_transform.translation + (direction * 0.7);
                            translation[1] = 1.0;

                            let fireball_entity = spawn_bullet(&mut commands, &mut texture_atlases, &asset_server, translation, direction);
                            let message = ServerMessages::SpawnProjectile {
                                entity: fireball_entity,
                                translation: [translation[0], translation[1]],
                            };
                            let message = bincode::serialize(&message).unwrap();
                            server.broadcast_message(ServerChannel::ServerMessages.id(), message);
                        }
                    }
                }
            }
        }
        while let Some(message) = server.receive_message(client_id, ClientChannel::Input.id()) {
            let input: PlayerInput = bincode::deserialize(&message).unwrap();
            client_ticks.0.insert(client_id, input.most_recent_tick);
            if let Some(player_entity) = lobby.players.get(&client_id) {
                commands.entity(*player_entity).insert(input);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn server_network_sync(
    mut tick: ResMut<NetworkTick>,
    mut server: ResMut<RenetServer>,
    networked_entities: Query<(Entity, &Transform), Or<(With<Player>, With<Projectile>)>>,
) {
    let mut frame = NetworkFrame::default();
    for (entity, transform) in networked_entities.iter() {
        frame.entities.entities.push(entity);
        frame.entities.translations.push(transform.translation.into());
    }

    frame.tick = tick.0;
    tick.0 += 1;
    let sync_message = bincode::serialize(&frame).unwrap();
    server.broadcast_message(ServerChannel::NetworkFrame.id(), sync_message);
}

// TODO
fn move_players_system(mut query: Query<(&mut Transform, &PlayerInput)>) {
    for (mut transform, input) in query.iter_mut() {
        let x = (input.right as i8 - input.left as i8) as f32;
        let y = (input.down as i8 - input.up as i8) as f32;
        let direction = Vec2::new(x, y).normalize_or_zero();
        transform.translation.x += direction.x * 0.1;
        transform.translation.y += direction.y * 0.1;
    }
}

fn update_projectiles_system(mut commands: Commands, mut projectiles: Query<(Entity, &mut Projectile)>, time: Res<Time>) {
    for (entity, mut projectile) in projectiles.iter_mut() {
        projectile.duration.tick(time.delta());
        if projectile.duration.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn projectile_on_removal_system(mut server: ResMut<RenetServer>, removed_projectiles: RemovedComponents<Projectile>) {
    for entity in removed_projectiles.iter() {
        let message = ServerMessages::DespawnProjectile { entity };
        let message = bincode::serialize(&message).unwrap();

        server.broadcast_message(ServerChannel::ServerMessages.id(), message);
    }
}