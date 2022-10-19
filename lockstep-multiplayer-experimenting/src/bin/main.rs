use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

use bevy::app::{App, AppExit, CoreStage};
use bevy::DefaultPlugins;
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy::window::WindowSettings;
use bevy_renet::{RenetClientPlugin, RenetServerPlugin, run_if_client_connected};
use iyes_loopless::prelude::*;
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetError, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};

use lockstep_multiplayer_experimenting::{AMOUNT_PLAYERS, client_connection_config, ClientChannel, ClientLobby, ClientTicks, ClientType, NetworkMapping, Player, PlayerId, PORT, PROTOCOL_ID, server_connection_config, ServerChannel, ServerLobby, ServerTick, Tick, TICKRATE, translate_host, translate_port, Username, VERSION};
use lockstep_multiplayer_experimenting::client_functionality::{client_update_system, new_renet_client};
use lockstep_multiplayer_experimenting::commands::PlayerCommand;
use lockstep_multiplayer_experimenting::server_functionality::{new_renet_server, server_update_system};
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

    let mut username = Player::default_username().0;
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

    /*
        Defines the tick the server is on currently
        The client isn't yet on this tick, it's the target tick.
     */
    app.insert_resource(ServerTick::new());

    match my_type {
        ClientType::Server => {
            app.insert_resource(new_renet_server(amount_of_players, host, port));
            app.insert_resource(ClientTicks::default());
            app.insert_resource(ServerLobby::default());
            app.add_system(server_update_system);

            let mut fixed_update_server = SystemStage::parallel();
            fixed_update_server.add_system(
                fixed_time_step
                    // only do it in-game
                    // .run_if(run_if_client_connected)
                    .run_if(run_if_tick_in_sync)
                    .run_if(run_if_enough_players)
            );

            app.add_stage_before(
                CoreStage::Update,
                "FixedUpdate",
                FixedTimestepStage::from_stage(Duration::from_millis(TICKRATE), fixed_update_server),
            );
        }
        ClientType::Client => {}
    }

    app.add_system(client_update_system);

    app.insert_resource(new_renet_client(&username, host, port));
    app.insert_resource(ClientLobby::default());
    app.insert_resource(Tick(Some(0)));
    app.insert_resource(NetworkMapping::default());

    app.run();
}

fn run_if_enough_players(
    lobby: Res<ServerLobby>,
) -> bool {
    if lobby.0.len() >= AMOUNT_PLAYERS {
        true
    } else {
        println!("Current amount of players: {}, needed amount of players: {}", lobby.0.len(), AMOUNT_PLAYERS);
        false
    }
}

fn run_if_tick_in_sync(
    tick: Res<Tick>,
    server_tick: Res<ServerTick>,
    client_ticks: Res<ClientTicks>,
    lobby: Res<ServerLobby>,
) -> bool {
    let mut client_iter = client_ticks.0.iter().peekable();
    while let Some((client_id, client_tick)) = client_iter.next() {
        if client_tick.get() != server_tick.get() {
            let username = lobby.0.get(&client_id).unwrap().username.clone();
            println!("Waiting for Client {}!", username);
            return false;
        }
    }

    true
}

fn fixed_time_step(
    // Client/All
    mut tick: ResMut<Tick>,
    mut client: ResMut<RenetClient>,
    mut server_tick: ResMut<ServerTick>,
    // Server
    mut server: Option<ResMut<RenetServer>>,
    mut client_ticks: Option<ResMut<ClientTicks>>,
    mut lobby: Option<ResMut<ServerLobby>>,
) {
    if let Some(server) = server.as_mut() { // we're server
        let server_tick = server_tick.as_mut();
        println!("All clients ready!");
        server_tick.increment();
        println!("Server Tick: {}", server_tick.get());

        let message = bincode::serialize(&UpdateTick {
            target_tick: server_tick.0,
        }).unwrap();
        server.broadcast_message(ServerChannel::ServerMessages.id(), message);
    }
}

////////// RENET NETWORKING //////////
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