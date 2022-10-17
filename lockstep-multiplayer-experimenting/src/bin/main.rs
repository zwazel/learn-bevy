use std::fmt::{Debug, Formatter};
use std::net::UdpSocket;
use std::time::SystemTime;
use bevy::app::{App, AppExit, CoreStage};
use bevy::DefaultPlugins;
use bevy::prelude::{ClearColor, Color, default, EventReader, ResMut, WindowDescriptor};
use bevy::window::WindowSettings;
use bevy_renet::{RenetClientPlugin, run_if_client_connected};
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetError};
use lockstep_multiplayer_experimenting::{client_connection_config, PORT, PROTOCOL_ID, translate_host, translate_port, VERSION};

enum Type {
    Client,
    Server,
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Client => write!(f, "Client"),
            Type::Server => write!(f, "Server"),
        }
    }
}


fn resolve_type(my_type: &str) -> Type {
    match my_type {
        "client" => Type::Client,
        "server" => Type::Server,
        _ => panic!("Invalid type"),
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    let mut username = format!("Player_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let mut host = "127.0.0.1";
    let mut port = PORT;
    let mut my_type = Type::Client;
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
        5 => {
            username = args[1].clone();
            host = translate_host(&args[2], "");
            port = translate_port(&args[3]);
            my_type = resolve_type(&args[4]);
            println!("Port has been set to: {}, Host has been set to: {}, Username has been set to: {}, Type has been set to: {:?}", port, host, username, my_type);
        }
        _ => {
            println!("Usage: client [username] [host] [port]");
            println!("Default values: username: {}, host: {}, port: {}", username, host, port);
        }
    }

    println!("Version: {}", VERSION);

    App::new()
        .insert_resource(WindowDescriptor {
            title: format!("Lockstep Experimenting <{}>", username),
            width: 480.0,
            height: 540.0,
            ..default()
        })
        .insert_resource(WindowSettings {
            ..default()
        })
        .insert_resource(new_renet_client(&username, host, port))

        .add_plugins(DefaultPlugins)
        .add_plugin(RenetClientPlugin)

        .add_system(panic_on_error_system)

        .add_system_to_stage(CoreStage::Last, disconnect)

        .run();
}

////////// RENET NETWORKING //////////
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