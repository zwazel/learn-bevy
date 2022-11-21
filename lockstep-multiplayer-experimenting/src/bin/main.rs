use std::{env};
use std::borrow::BorrowMut;

use std::fmt::{Debug};
use std::fs::{create_dir_all, File};
use std::io::{Write};


use std::time::{Duration};

use bevy::app::{App, AppExit, CoreStage};
use bevy::DefaultPlugins;
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;

use bevy::window::PresentMode;

use bevy_asset_loader::prelude::*;
use bevy_mod_picking::{DebugCursorPickingPlugin, DefaultPickingPlugins, PickingCameraBundle};
use bevy_rapier3d::prelude::*;
use bevy_renet::{RenetClientPlugin, RenetServerPlugin, run_if_client_connected};

use iyes_loopless::prelude::*;

use renet::{RenetClient, RenetError};


use lockstep_multiplayer_experimenting::{AMOUNT_PLAYERS, CameraLight, CameraMovement, CameraSettings, ClientLobby, ClientTicks, ClientType, CurrentServerTick, GameState, LocalServerTick, MainCamera, NetworkMapping, Player, PlayerId, PORT, SAVE_REPLAY, ServerLobby, ServerMarker, Tick, TICKRATE, translate_host, translate_port, VERSION};
use lockstep_multiplayer_experimenting::client_functionality::{client_update_system, create_new_units, fixed_time_step_client, interpolate_movement_of_other_players, move_camera, move_units, new_renet_client, place_move_target};
use lockstep_multiplayer_experimenting::commands::{CommandQueue, MyDateTime, ServerSyncedPlayerCommandsList, SyncedPlayerCommandsList};
use lockstep_multiplayer_experimenting::entities::Target;
use lockstep_multiplayer_experimenting::physic_stuff::PlaceableSurface;
use lockstep_multiplayer_experimenting::server_functionality::{fixed_time_step_server, new_renet_server, server_update_system};



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

#[derive(Resource)]
struct SaveReplay(bool);

fn main() {
    // env::set_var("RUST_BACKTRACE", "full");

    let args = env::args().collect::<Vec<String>>();

    let mut username = Player::default_username().0;
    let mut host = "127.0.0.1";
    let mut port = PORT;
    let mut my_type = ClientType::Client;
    let mut amount_of_players = AMOUNT_PLAYERS;
    let mut tickrate = TICKRATE;
    let mut save_replay = SAVE_REPLAY;

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
        7 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();
            amount_of_players = translate_amount_players(&args[3]);
            host = translate_host(&args[4], "");
            port = translate_port(&args[5]);
            tickrate = args[6].parse::<u64>().unwrap_or(TICKRATE);

            println!("Type has been set to: {}, Username has been set to: {}, Amount of players has been set to: {}, Host has been set to: {}, Port has been set to: {}, Tickrate has been set to: {}", my_type, username, amount_of_players, host, port, tickrate);
        }
        8 => {
            my_type = resolve_type(&args[1]);
            username = args[2].clone();
            amount_of_players = translate_amount_players(&args[3]);
            host = translate_host(&args[4], "");
            port = translate_port(&args[5]);
            tickrate = args[6].parse::<u64>().unwrap_or(TICKRATE);
            save_replay = args[7].parse::<bool>().unwrap_or(SAVE_REPLAY);

            println!("Type has been set to: {}, Username has been set to: {}, Amount of players has been set to: {}, Host has been set to: {}, Port has been set to: {}, Tickrate has been set to: {}, save replay has been set to: {}", my_type, username, amount_of_players, host, port, tickrate, save_replay);
        }
        _ => {
            println!("Usage: client [ClientType: server/client] [username] [host] [port] [amount of players] [tickrate] [save replay]");
            println!("Default values:\n\tClientType: {}\n\tusername: {}\n\thost: {}\n\tport: {}\n\tamount players: {}\n\ttickrate: {}\n\tsave replay: {}", my_type, username, host, port, amount_of_players, tickrate, save_replay);
        }
    }

    println!("Version: {}", VERSION);

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        window: WindowDescriptor {
            title: format!("Lockstep Experimenting <{}>", username),
            width: 480.0,
            height: 540.0,
            present_mode: PresentMode::AutoNoVsync, // Reduce input latency
            ..default()
        },
        ..default()
    }));
    app.add_plugin(RenetClientPlugin::default());
    app.add_plugin(RenetServerPlugin::default());
    app.add_plugins(DefaultPickingPlugins); // <- Adds Picking, Interaction, and Highlighting plugins.
    app.add_plugin(DebugCursorPickingPlugin); // <- Adds the green debug cursor.
    // app.add_plugin(DebugEventsPickingPlugin); // <- Adds debug event logging.
    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugin(RapierDebugRenderPlugin::default());

    app.add_system(panic_on_error_system);
    app.add_system_to_stage(CoreStage::Last, disconnect);

    /*
        Defines the tick the server is on currently
        The client isn't yet on this tick, it's the target tick.
     */
    app.insert_resource(LocalServerTick::new());
    app.insert_resource(SyncedPlayerCommandsList::default());
    app.insert_resource(CommandQueue::default());
    app.insert_resource(SaveReplay(save_replay));

    // app.add_loading_state(
    //     LoadingState::new(GameState::Loading)
    //         .continue_to_state(GameState::InGame)
    //         .with_collection::<TargetAssets>()
    //         .with_collection::<UnitAssets>()
    // );
    app.add_state(GameState::InGame);
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_scene);

    match my_type {
        ClientType::Server => {
            app.insert_resource(new_renet_server(amount_of_players, host, port));
            app.insert_resource(CurrentServerTick::new());
            app.insert_resource(ClientTicks::default());
            app.insert_resource(ServerLobby::default());
            app.insert_resource(ServerMarker);
            app.insert_resource(AmountPlayers(amount_of_players));
            app.insert_resource(ServerSyncedPlayerCommandsList::default());
            app.add_system(server_update_system);

            let mut fixed_update_server = SystemStage::parallel();
            fixed_update_server.add_system_set(
                SystemSet::on_update(GameState::InGame)
                    .with_system(fixed_time_step_server
                        .ambiguous_with(fixed_time_step_client)
                    )
                    .with_run_criteria(run_server_time_step_if_in_sync)
            );

            app.add_stage_before(
                CoreStage::Update,
                "FixedUpdateServer",
                FixedTimestepStage::from_stage(Duration::from_millis(tickrate), "FixedServerUpdate", fixed_update_server),
            );
        }
        _ => {}
    }

    let mut fixed_update_client = SystemStage::parallel();
    fixed_update_client.add_system_set(
        SystemSet::on_update(GameState::InGame)
            .with_system(fixed_time_step_client)
            .with_run_criteria(run_if_tick_in_sync_client)
    );
    app.add_stage_before(
        CoreStage::Update,
        "FixedUpdateClient",
        FixedTimestepStage::from_stage(Duration::from_millis(tickrate), "FixedClientUpdate", fixed_update_client),
    );

    app.add_system(move_camera
        .label(MySystems::Movement)
        .before(MySystems::Syncing)
        .before(MySystems::CommandCollection)
    );

    app.add_system_set(
        SystemSet::on_update(GameState::InGame)
            .with_system(
                client_update_system
                    .label(MySystems::Syncing)
                    .after(MySystems::CommandCollection)
            )
            .with_system(
                fade_away_targets
            )
            .with_system(
                move_units
            )
            .with_system(
                interpolate_movement_of_other_players
            )
            .with_system(
                create_new_units
                    .label(MySystems::CommandCollection)
            ).with_system(
            place_move_target
                .label(MySystems::CommandCollection)
        )
            .with_run_criteria(run_if_client_connected)
    );

    app.add_system_set(
        SystemSet::on_exit(GameState::Loading)
            .with_system(loading_informer)
            .with_system(setup_ui)
    );

    app.insert_resource(new_renet_client(&username, host, port));
    app.insert_resource(ClientLobby::default());
    app.insert_resource(Tick(0));
    app.insert_resource(NetworkMapping::default());
    app.insert_resource(CameraMovement::default());
    app.insert_resource(CameraSettings::default());

    app.run();
}

fn setup_ui() {
    println!("Setting up UI");
}

fn loading_informer() {
    println!("Loading finished");
}

#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
enum MySystems {
    CommandCollection,
    Syncing,
    Movement,
}

#[derive(Resource)]
struct AmountPlayers(usize);

fn setup_camera(
    mut commands: Commands,
) {
    let spatial_bundle = commands
        .spawn(SpatialBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0),
            ..default()
        })
        .insert(MainCamera)
        .id();

    // camera
    let camera = commands
        .spawn(Camera3dBundle::default())
        .insert(PickingCameraBundle::default())
        .insert(MainCamera)
        .id();

    let light = commands
        .spawn(SpotLightBundle {
            spot_light: SpotLight {
                range: 500.0,
                intensity: 1000.0,
                shadows_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CameraLight)
        .id();

    commands.entity(spatial_bundle).push_children(&[camera, light]);
}

fn setup_scene(mut commands: Commands,
               mut meshes: ResMut<Assets<Mesh>>,
               mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    let floor_size = 20.0;
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: floor_size })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    })
        .with_children(|children| {
            children
                .spawn(Collider::cuboid(floor_size / 2.0, 0.0, floor_size / 2.0))
                .insert(CollisionGroups::new(Group::GROUP_2, Group::GROUP_2))
                .insert_bundle(TransformBundle {
                    ..Default::default()
                });
        })
        .insert(PlaceableSurface);
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

fn run_server_time_step_if_in_sync(
    server_tick: Res<CurrentServerTick>,
    client_ticks: Res<ClientTicks>,
    lobby: Res<ServerLobby>,
    amount_players: Res<AmountPlayers>,
) -> ShouldRun {
    if lobby.0.len() < amount_players.0 {
        println!("Current amount of players: {}, needed amount of players: {}", lobby.0.len(), amount_players.0);
        return ShouldRun::No;
    }

    let mut client_iter = client_ticks.0.iter().peekable();
    let mut players_synced = true;
    while let Some((client_id, client_tick)) = client_iter.next() {
        if client_tick.get() != server_tick.get() {
            let _username = lobby.0.get(&client_id).unwrap().username.clone();
            // println!("Waiting for Client {}!", username);
            players_synced = false;
        }
    }

    if !players_synced {
        return ShouldRun::No;
    }

    ShouldRun::Yes
}

fn run_if_tick_in_sync_client(
    server_tick: Res<LocalServerTick>,
    client_tick: ResMut<Tick>,
) -> ShouldRun {
    let future_client_tick = client_tick.get() + 1;
    if future_client_tick == server_tick.get() {
        ShouldRun::Yes
    } else {
        // println!("Waiting for Server! Current Client Tick: {}, Target Client Tick: {}, Server Tick: {}", client_tick.get(), future_client_tick, server_tick.get());
        ShouldRun::No
    }
}

////////// RENET NETWORKING //////////
fn disconnect(
    mut events: EventReader<AppExit>,
    mut client: ResMut<RenetClient>,
    client_lobby: Option<Res<ClientLobby>>,
    mut command_history: ResMut<SyncedPlayerCommandsList>,
    is_server: Option<Res<ServerMarker>>,
    save_replay: Res<SaveReplay>,
) {
    if let Some(_) = events.iter().next() {
        if save_replay.0 {
            if let Some(client_lobby) = client_lobby.as_ref() {
                let client_lobby = client_lobby.as_ref();
                let username = client_lobby.get_username(PlayerId(client.client_id())).unwrap();
                save_replays(username, command_history.borrow_mut());
            }
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
