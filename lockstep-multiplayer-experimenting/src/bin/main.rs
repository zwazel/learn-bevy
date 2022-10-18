use std::fmt::{Debug, Display, Formatter};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};
use bevy::app::{App, AppExit, CoreStage};
use bevy::DefaultPlugins;
use bevy::prelude::*;
use bevy::window::WindowSettings;
use bevy_renet::{RenetClientPlugin, RenetServerPlugin, run_if_client_connected};
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetError, RenetServer, ServerAuthentication, ServerConfig};
use lockstep_multiplayer_experimenting::{AMOUNT_PLAYERS, client_connection_config, PORT, PROTOCOL_ID, server_connection_config, TICKRATE, translate_host, translate_port, VERSION};
use iyes_loopless::prelude::*;

struct Tick(i128);

enum NetworkType {
    Client,
    Server,
}

impl Debug for NetworkType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkType::Client => write!(f, "Client"),
            NetworkType::Server => write!(f, "Server"),
        }
    }
}

impl Display for NetworkType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkType::Client => write!(f, "Client"),
            NetworkType::Server => write!(f, "Server"),
        }
    }
}

fn resolve_type(my_type: &str) -> NetworkType {
    let my_type = my_type.to_lowercase();
    match my_type.as_str() {
        "client" => NetworkType::Client,
        "server" => NetworkType::Server,
        _ => NetworkType::Client,
    }
}

fn translate_amount_players(amount_players: &str) -> usize {
    amount_players.parse::<usize>().unwrap_or(AMOUNT_PLAYERS)
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    let mut username = format!("Player_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let mut host = "127.0.0.1";
    let mut port = PORT;
    let mut my_type = NetworkType::Client;
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
        FixedTimestepStage::from_stage(Duration::from_millis(TICKRATE), fixed_update)
    );

    match my_type {
        NetworkType::Server => {
            app.insert_resource(new_renet_server(amount_of_players, host, port));
        }
        _ => {}
    }

    app.insert_resource(new_renet_client(&username, host, port));

    app.insert_resource(Tick(0));

    app.run();
}

fn fixed_time_step(
    mut tick: ResMut<Tick>,
) {
    tick.0 += 1;
    println!("Tick: {}", tick.0);
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