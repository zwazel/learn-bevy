use std::{env, f32};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::app::{App, PluginGroup, PluginGroupBuilder};
use bevy::prelude::*;
use bevy_renet::RenetServerPlugin;
use renet::{NETCODE_USER_DATA_BYTES, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};

use vampire_surviors_clone::{AMOUNT_PLAYERS, ClientChannel, MaxSpeed, NetworkFrame, Player, PLAYER_SPEED, PlayerCommand, PlayerInput, PORT, PROTOCOL_ID, server_connection_config, ServerChannel, ServerMessages, translate_host, translate_port, Velocity};

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
    app.add_plugins(ServerPlugins);
    app.add_plugin(RenetServerPlugin);

    app.insert_resource(ServerLobby::default());
    app.insert_resource(NetworkTick(0));
    app.insert_resource(ClientTicks::default());
    app.insert_resource(new_renet_server(amount_of_players, host, port));

    app.add_system(server_update_system);
    app.add_system(server_network_sync);
    app.add_system(move_players_system);

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
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, user_data) => {
                let username = name_from_user_data(&user_data);
                println!("Player {} connected.", username);

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
                    .spawn()
                    .insert(Player { id: *id })
                    .insert(PlayerInput::default())
                    .insert(transform)
                    .insert(Velocity(Vec2::ZERO))
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
                PlayerCommand::BasicAttack { .. } => {}
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
    networked_entities: Query<(Entity, &Transform), With<Player>>,
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
fn move_players_system(mut query: Query<(&mut Transform, &PlayerInput, &mut Velocity, &MaxSpeed)>) {
    for (mut transform, input, mut velocity, max_speed) in query.iter_mut() {
        let x = (input.right as i8 - input.left as i8) as f32;
        let y = (input.down as i8 - input.up as i8) as f32;
        let direction = Vec2::new(x, y).normalize_or_zero();
        velocity.0.x = direction.x * PLAYER_SPEED;
        velocity.0.y = direction.y * PLAYER_SPEED;

        transform.translation.x += velocity.0.x;
        transform.translation.y += velocity.0.y;
    }
}

pub struct ServerPlugins;

impl PluginGroup for ServerPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(bevy::log::LogPlugin::default());
        group.add(bevy::core::CorePlugin::default());
        group.add(bevy::time::TimePlugin::default());
        group.add(TransformPlugin::default());
        group.add(HierarchyPlugin::default());
        group.add(bevy::diagnostic::DiagnosticsPlugin::default());
        group.add(bevy::app::ScheduleRunnerPlugin::default());
    }
}