use std::{env, fs};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::fs::{create_dir, create_dir_all, File, write};
use std::io::{Read, Write};
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;
use std::time::{Duration, SystemTime};

use bevy::app::{App, AppExit, CoreStage};
use bevy::DefaultPlugins;
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy::reflect::GetPath;
use bevy::window::WindowSettings;
use bevy_renet::{RenetClientPlugin, RenetServerPlugin, run_if_client_connected};
use chrono::{DateTime, Utc};
use iyes_loopless::prelude::*;
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetError, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};
use serde_json::json;

use lockstep_multiplayer_experimenting::{AMOUNT_PLAYERS, client_connection_config, ClientChannel, ClientLobby, ClientTicks, ClientType, GameState, MainCamera, NetworkMapping, Player, PlayerId, PORT, PROTOCOL_ID, server_connection_config, ServerChannel, ServerLobby, ServerMarker, ServerTick, Target, Tick, TICKRATE, translate_host, translate_port, Username, VERSION};
use lockstep_multiplayer_experimenting::client_functionality::{client_update_system, handle_mouse_input, new_renet_client};
use lockstep_multiplayer_experimenting::commands::{CommandQueue, MyDateTime, PlayerCommand, PlayerCommandsList, ServerSyncedPlayerCommandsList, SyncedPlayerCommand, SyncedPlayerCommandsList};
use lockstep_multiplayer_experimenting::server_functionality::{new_renet_server, server_update_system};
use lockstep_multiplayer_experimenting::ServerChannel::ServerMessages;
use lockstep_multiplayer_experimenting::ServerMessages::{PlayerCreate, PlayerRemove, UpdateTick};

use bevy_asset_loader::prelude::*;
use lockstep_multiplayer_experimenting::asset_handling::TargetAssets;

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
    // env::set_var("RUST_BACKTRACE", "full");

    let args = env::args().collect::<Vec<String>>();

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
            amount_of_players = translate_amount_players(&args[3]);

            println!("Type has been set to: {}, Username has been set to: {}, Amount of players has been set to: {}", my_type, username, amount_of_players);
        }
        5 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();
            amount_of_players = translate_amount_players(&args[3]);
            host = translate_host(&args[4], "");

            println!("Type has been set to: {}, Username has been set to: {}, Amount of Players has been set to: {}, Host has been set to: {}", my_type, username, amount_of_players, host);
        }
        6 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();
            amount_of_players = translate_amount_players(&args[5]);
            host = translate_host(&args[4], "");
            port = translate_port(&args[5]);

            println!("Type has been set to: {}, Username has been set to: {}, Amount of players has been set to: {}, Host has been set to: {}, Port has been set to: {}", my_type, username, amount_of_players, host, port);
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
    app.insert_resource(SyncedPlayerCommandsList::default());
    app.insert_resource(CommandQueue::default());

    app.add_loading_state(
        LoadingState::new(GameState::Loading)
            .continue_to_state(GameState::InGame)
            .with_collection::<TargetAssets>()
    );
    app.add_state(GameState::Loading);
    app.add_startup_system(setup_camera);

    match my_type {
        ClientType::Server => {
            app.insert_resource(new_renet_server(amount_of_players, host, port));
            app.insert_resource(ClientTicks::default());
            app.insert_resource(ServerLobby::default());
            app.insert_resource(ServerMarker);
            app.insert_resource(AmountPlayers(amount_of_players));
            app.insert_resource(ServerSyncedPlayerCommandsList::default());
            app.add_system(server_update_system);

            let mut fixed_update_server = SystemStage::parallel();
            fixed_update_server.add_system_set(
                SystemSet::on_update(GameState::InGame)
                    .with_system(fixed_time_step)
                    .with_run_criteria(run_if_tick_in_sync)
                    .with_run_criteria(run_if_enough_players)
            );

            app.add_stage_before(
                CoreStage::Update,
                "FixedUpdate",
                FixedTimestepStage::from_stage(Duration::from_millis(TICKRATE), fixed_update_server),
            );
        }
        _ => {}
    }

    app.add_system_set(
        SystemSet::on_update(GameState::InGame)
            .with_system(
                handle_mouse_input
                    .before(MySystems::Syncing)
                    .label(MySystems::CommandCollection)
            )
            .with_system(
                client_update_system
                    .label(MySystems::Syncing)
                    .after(MySystems::CommandCollection)
            )
            .with_system(
                fade_away_targets
            )
            .with_run_criteria(run_if_client_connected)
    );

    app.add_system_set(
        SystemSet::on_exit(GameState::Loading)
            .with_system(loading_informer)
    );

    app.insert_resource(new_renet_client(&username, host, port));
    app.insert_resource(ClientLobby::default());
    app.insert_resource(Tick(0));
    app.insert_resource(NetworkMapping::default());

    app.run();
}

fn loading_informer() {
    println!("Loading finished");
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(SystemLabel)]
enum MySystems {
    CommandCollection,
    Syncing,
}

struct AmountPlayers(usize);

fn run_if_enough_players(
    lobby: Res<ServerLobby>,
    amount_players: Res<AmountPlayers>,
) -> ShouldRun {
    if lobby.0.len() >= amount_players.0 {
        ShouldRun::Yes
    } else {
        println!("Current amount of players: {}, needed amount of players: {}", lobby.0.len(), amount_players.0);
        ShouldRun::No
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn()
        .insert_bundle(Camera2dBundle::default())
        .insert(MainCamera);
}

fn fade_away_targets(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Sprite), With<Target>>,
) {
    for (entity, mut sprite) in query.iter_mut() {
        if sprite.color.a() <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            let current_alpha = sprite.color.a();
            sprite.color.set_a(current_alpha - 0.01);
        }
    }
}

fn run_if_tick_in_sync(
    server_tick: Res<ServerTick>,
    client_ticks: Res<ClientTicks>,
    lobby: Res<ServerLobby>,
) -> ShouldRun {
    let mut client_iter = client_ticks.0.iter().peekable();
    let mut players_synced = true;
    while let Some((client_id, client_tick)) = client_iter.next() {
        if client_tick.get() != server_tick.get() {
            let username = lobby.0.get(&client_id).unwrap().username.clone();
            println!("Waiting for Client {}!", username);
            players_synced = false;
        }
    }

    return if players_synced {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    };
}

fn fixed_time_step(
    // Client/All
    mut server_tick: ResMut<ServerTick>,
    mut synced_commands: ResMut<ServerSyncedPlayerCommandsList>,
    // Server
    mut server: Option<ResMut<RenetServer>>,
) {
    if let Some(server) = server.as_mut() { // we're server
        let server_tick = server_tick.as_mut();

        let commands = synced_commands.0.0.get(&Tick(server_tick.get()));

        server_tick.increment();

        let message = bincode::serialize(&UpdateTick {
            target_tick: server_tick.0,
            commands: {
                if let Some(commands) = commands {
                    commands.clone()
                } else {
                    SyncedPlayerCommand::default()
                }
            },
        }).unwrap();

        synced_commands.0.0.insert(server_tick.0, SyncedPlayerCommand(PlayerCommandsList::default(), MyDateTime::now()));

        server.broadcast_message(ServerChannel::ServerTick.id(), message);
    }
}

////////// RENET NETWORKING //////////
fn disconnect(
    mut events: EventReader<AppExit>,
    mut client: ResMut<RenetClient>,
    client_lobby: Option<Res<ClientLobby>>,
    mut command_history: ResMut<SyncedPlayerCommandsList>,
    is_server: Option<Res<ServerMarker>>,
) {
    if let Some(_) = events.iter().next() {
        if let Some(client_lobby) = client_lobby.as_ref() {
            let client_lobby = client_lobby.as_ref();
            let username = client_lobby.get_username(PlayerId(client.client_id())).unwrap();
            save_replays(username, command_history.borrow_mut());
        }

        if let Some(_) = is_server {
            println!("Server Stopped!");
        } else {
            println!("Client disconnected!");
        }

        client.disconnect();
        std::process::exit(0);
    }
}

fn save_replays(username: String, command_history: &mut SyncedPlayerCommandsList) {
    command_history.remove_empty();

    if !command_history.is_empty() {
        let mut replay_dir = env::current_dir().unwrap();
        replay_dir.push("replays");
        replay_dir.push(username);
        create_dir_all(&replay_dir).unwrap();

        replay_dir.push(format!("replay_{}.json", MyDateTime::now().to_string()));
        let mut replay_file = File::create(&replay_dir).unwrap();

        replay_file.write_all(serde_json::to_string(command_history).unwrap().as_bytes()).unwrap();

        println!("Saved replay to: {}", replay_dir.to_str().unwrap());
    }

    // // read created file
    // let mut replay_file = File::open(&replay_dir).unwrap();
    // let mut replay_file_contents = String::new();
    // replay_file.read_to_string(&mut replay_file_contents).unwrap();
    //
    // // deserialize
    // let replay: SyncedPlayerCommandsList = serde_json::from_str(&replay_file_contents).unwrap();
    // println!("Replay: {}", replay);
}

// If any error is found we just panic
fn panic_on_error_system(mut renet_error: EventReader<RenetError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}