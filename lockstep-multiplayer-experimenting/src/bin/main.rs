use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};
use bevy::app::{App, AppExit, CoreStage};
use bevy::DefaultPlugins;
use bevy::prelude::*;
use bevy::window::WindowSettings;
use bevy_renet::{RenetClientPlugin, RenetServerPlugin, run_if_client_connected};
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetError, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};
use lockstep_multiplayer_experimenting::{AMOUNT_PLAYERS, client_connection_config, ClientChannel, ClientTicks, ClientType, Player, PlayerId, PORT, PROTOCOL_ID, server_connection_config, ServerChannel, Lobby, Tick, TICKRATE, translate_host, translate_port, VERSION, ServerTick};
use iyes_loopless::prelude::*;
use lockstep_multiplayer_experimenting::commands::PlayerCommand;
use lockstep_multiplayer_experimenting::ServerChannel::ServerMessages;
use lockstep_multiplayer_experimenting::ServerMessages::{PlayerCreate, PlayerRemove, UpdateTick};

fn resolve_type(my_type: &str) -> ClientType {
    let my_type = my_type.to_lowercase();
    match my_type.as_str() {
        "client" => ClientType::Client,
        "server" => ClientType::Server,
        _ => ClientType::Client,
    }
}

fn translate_amount_players(amount_players: &str) -> usize {
    amount_players.parse::<usize>().unwrap_or(AMOUNT_PLAYERS)
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    let mut username = Player::default_username();
    let mut host = "127.0.0.1";
    let mut port = PORT;
    let mut my_type = ClientType::Client;
    let mut amount_of_players = AMOUNT_PLAYERS;
    match args.len() {
        2 => {
            my_type = resolve_type(&args[1]);

            println!("Type has been set to {}", my_type);
        }
        3 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();

            println!("Type has been set to {}, Username has been set to: {}", my_type, username);
        }
        4 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();
            host = translate_host(&args[3], "");

            println!("Type has been set to: {}, Username has been set to: {}, Host has been set to: {}", my_type, username, host);
        }
        5 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();
            host = translate_host(&args[3], "");
            port = translate_port(&args[4]);

            println!("Type has been set to: {}, Username has been set to: {}, Host has been set to: {}, Port has been set to: {}", my_type, username, host, port);
        }
        6 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();
            host = translate_host(&args[3], "");
            port = translate_port(&args[4]);
            amount_of_players = translate_amount_players(&args[5]);

            println!("Type has been set to: {}, Username has been set to: {}, Host has been set to: {}, Port has been set to: {}, Amount of players has been set to: {}", my_type, username, host, port, amount_of_players);
        }
        _ => {
            println!("Usage: client [ClientType: server/client] [username] [host] [port] [amount of players]");
            println!("Default values:\n\tClientType: {}\n\tusername: {}\n\thost: {}\n\tport: {}\n\tamount players: {}", my_type, username, host, port, amount_of_players);
        }
    }

    println!("Version: {}", VERSION);

    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        title: format!("Lockstep Experimenting <{}>", username),
        width: 480.0,
        height: 540.0,
        ..default()
    });
    app.insert_resource(WindowSettings {
        ..default()
    });

    app.add_plugins(DefaultPlugins);
    app.add_plugin(RenetServerPlugin);
    app.add_plugin(RenetClientPlugin);

    app.add_system(panic_on_error_system);
    app.add_system_to_stage(CoreStage::Last, disconnect);

    let mut fixed_update = SystemStage::parallel();
    fixed_update.add_system(
        fixed_time_step
            // only do it in-game
            .with_run_criteria(run_if_client_connected)
    );

    app.add_stage_before(
        CoreStage::Update,
        "FixedUpdate",
        FixedTimestepStage::from_stage(Duration::from_millis(TICKRATE), fixed_update),
    );

    match my_type {
        ClientType::Server => {
            app.insert_resource(new_renet_server(amount_of_players, host, port));
            app.insert_resource(ClientTicks::default());
            app.insert_resource(Lobby::default());
            app.insert_resource(ServerTick::new());
            app.add_system(server_update_system);
        }
        ClientType::Client => {}
    }

    app.insert_resource(new_renet_client(&username, host, port));

    app.insert_resource(Tick(Some(0)));

    app.run();
}

fn fixed_time_step(
    // Client
    mut tick: ResMut<Tick>,
    mut client: ResMut<RenetClient>,
    // Server
    mut server: Option<ResMut<RenetServer>>,
    mut client_ticks: Option<ResMut<ClientTicks>>,
    mut server_tick: Option<ResMut<ServerTick>>,
    mut lobby: Option<ResMut<Lobby>>,
) {
    if let Some(server) = server.as_mut() {
        let server_tick = server_tick.as_mut().unwrap();
        if let Some(client_ticks) = client_ticks.as_mut() {
            let mut client_iter = client_ticks.0.iter().peekable();
            let mut clients_ready = client_iter.len() > 0;
            while let Some((client_id, client_tick)) = client_iter.next() {
                if client_tick.get() != server_tick.get() {
                    let username = lobby.as_ref().unwrap().0.get(&client_id).unwrap().username.clone();
                    println!("Waiting for Client {}!", username);
                    clients_ready = false;
                }
            }

            if clients_ready {
                println!("All clients ready!");
                server_tick.increment();
                println!("Server Tick: {}", server_tick.get());

                let message = bincode::serialize(&UpdateTick {
                    tick: server_tick.0,
                }).unwrap();
                server.broadcast_message(ServerChannel::ServerMessages.id(), message);
            }
        }
    }
}

////////// RENET NETWORKING //////////
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

fn new_renet_client(username: &String, host: &str, port: i32) -> RenetClient {
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

#[allow(clippy::too_many_arguments)]
fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<Lobby>,
    mut server: ResMut<RenetServer>,
    mut client_ticks: ResMut<ClientTicks>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, user_data) => {
                let username = name_from_user_data(&user_data);
                println!("Player {} connected.", username);

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
                    id: PlayerId(*id),
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
        // while let Some(message) = server.receive_message(client_id, ClientChannel::Input.id()) {
        //     let input: PlayerInput = bincode::deserialize(&message).unwrap();
        //     client_ticks.0.insert(PlayerId(client_id), input.most_recent_tick);
        //     if let Some(player_entity) = lobby.players.get(&client_id) {
        //         commands.entity(*player_entity).insert(input);
        //     }
        // }
    }
}

/// Utility function for extracting a players name from renet user data
fn name_from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> String {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&user_data[0..8]);
    let mut len = u64::from_le_bytes(buffer) as usize;
    len = len.min(NETCODE_USER_DATA_BYTES - 8);
    let data = user_data[8..len + 8].to_vec();
    String::from_utf8(data).unwrap()
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