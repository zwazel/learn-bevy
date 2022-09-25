#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget::Window;
use bevy::utils::HashMap;
use bevy::window::{WindowClosed, WindowCloseRequested, WindowPlugin, WindowSettings};
use bevy_renet::{RenetClientPlugin, run_if_client_connected};
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetConnectionConfig, RenetError};

use store::{GameEvent, GameState, HOST, PlayerId, PORT, Position, PROTOCOL_ID, Direction};

fn main() {
    // Get username from stdin args
    let args = std::env::args().collect::<Vec<String>>();

    let mut username = format!("Player_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    match args.len() {
        2 => {
            username = args[1].clone();
        }
        _ => {
            println!("Usage: client [username]");
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
        .add_plugins(DefaultPlugins)
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation
                    .label(RunPriority::Run)
                ),
        )
        // Renet setup
        .add_plugin(RenetClientPlugin)
        .insert_resource(new_renet_client(&username).unwrap())
        .insert_resource(PlayerHandles::default())
        .add_system(handle_renet_error
            .label(RunPriority::Run)
        )
        .add_system(move_entities
            .label(RunPriority::Run)
        )
        .add_system(move_input
            .label(RunPriority::Run)
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            receive_events_from_server
                .with_run_criteria(run_if_client_connected)
                .label(RunPriority::Run),
        )
        // Add our game state and register GameEvent as a bevy event
        .insert_resource(GameState::default())
        .add_event::<GameEvent>()
        // Add setup function to spawn UI and board graphics
        .add_startup_system(setup)
        // Finally we run the thing!
        .add_system_to_stage(CoreStage::Last, disconnect
            .label(RunPriority::Cleanup)
            .after(RunPriority::Run))
        .run();
}

#[derive(SystemLabel, Clone, Hash, Debug, PartialEq, Eq)]
enum RunPriority {
    Run,
    Cleanup,
}


////////// SETUP //////////
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());
}

// Components
#[derive(Component, Clone, Copy)]
struct PlayerHandle {
    client_id: PlayerId,
    entity: Entity,
    dir: Direction,
}

#[derive(Component)]
struct PlayerHandles {
    handles: HashMap<PlayerId, PlayerHandle>,
}

impl Default for PlayerHandles {
    fn default() -> Self {
        Self {
            handles: HashMap::new(),
        }
    }
}

#[derive(Component)]
struct Player;

#[derive(Component, Clone, Copy, PartialEq)]
struct ComponentPosition {
    pos: Position,
}

#[derive(Component)]
struct Name {
    name: String,
}

fn position_translation(windows: Res<Windows>,
                        mut q: Query<
                            (&ComponentPosition, &mut Transform),
                        >,
) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window_option = windows.get_primary();
    if let Some(window) = window_option {
        for (pos, mut transform) in q.iter_mut() {
            transform.translation = Vec3::new(
                convert(pos.pos.x, window.width(), 100.0),
                convert(pos.pos.y, window.height(), 100.0),
                0.0,
            );
        }
    }
}

fn move_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&PlayerHandle, &ComponentPosition), With<Player>>,
    mut client: ResMut<RenetClient>,
) {
    for (handle, pos) in query.iter_mut() {
        let handle: &PlayerHandle = handle;
        let pos: &ComponentPosition = pos;

        if keyboard_input.just_pressed(KeyCode::W) {
            let move_event = GameEvent::MovementKeyPressed {
                player_id: handle.client_id,
                direction: Direction::Up,
                start_pos: pos.pos,
            };

            println!("Sending event: {:?}", move_event);
            client.send_message(0, bincode::serialize(&move_event).unwrap());
        } else if keyboard_input.just_pressed(KeyCode::A) {
            let move_event = GameEvent::MovementKeyPressed {
                player_id: handle.client_id,
                direction: Direction::Left,
                start_pos: pos.pos,
            };

            println!("Sending event: {:?}", move_event);
            client.send_message(0, bincode::serialize(&move_event).unwrap());
        } else if keyboard_input.just_pressed(KeyCode::S) {
            let move_event = GameEvent::MovementKeyPressed {
                player_id: handle.client_id,
                direction: Direction::Down,
                start_pos: pos.pos,
            };

            println!("Sending event: {:?}", move_event);
            client.send_message(0, bincode::serialize(&move_event).unwrap());
        } else if keyboard_input.just_pressed(KeyCode::D) {
            let move_event = GameEvent::MovementKeyPressed {
                player_id: handle.client_id,
                direction: Direction::Right,
                start_pos: pos.pos,
            };

            println!("Sending event: {:?}", move_event);
            client.send_message(0, bincode::serialize(&move_event).unwrap());
        } else if keyboard_input.just_released(KeyCode::W)
            || keyboard_input.just_released(KeyCode::A)
            || keyboard_input.just_released(KeyCode::S)
            || keyboard_input.just_released(KeyCode::D)
        {
            let move_event = GameEvent::MovementKeyReleased {
                player_id: handle.client_id,
                position: pos.pos,
            };

            println!("Sending event: {:?}", move_event);
            client.send_message(0, bincode::serialize(&move_event).unwrap());
        }
    }
}

fn move_entities(
    mut query: Query<(&mut ComponentPosition, &PlayerHandle)>,
) {
    for (mut pos, handle) in query.iter_mut() {
        let dir = handle.dir as Direction;
        pos.pos.x = pos.pos.x + dir.value().x;
        pos.pos.y = pos.pos.y + dir.value().y;
    }
}

////////// RENET NETWORKING //////////
fn new_renet_client(username: &String) -> anyhow::Result<RenetClient> {
    let server_addr = format!("{}:{}", HOST, PORT).parse()?;
    let socket = UdpSocket::bind(format!("{}:0", HOST))?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;

    // Place username in user data
    let mut user_data = [0u8; NETCODE_USER_DATA_BYTES];
    if username.len() > NETCODE_USER_DATA_BYTES - 8 {
        panic!("Username is too big");
    }
    user_data[0..8].copy_from_slice(&(username.len() as u64).to_le_bytes());
    user_data[8..username.len() + 8].copy_from_slice(username.as_bytes());

    let client = RenetClient::new(
        current_time,
        socket,
        client_id,
        RenetConnectionConfig::default(),
        ClientAuthentication::Unsecure {
            client_id,
            protocol_id: PROTOCOL_ID,
            server_addr,
            user_data: Some(user_data),
        },
    )?;

    Ok(client)
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

fn receive_events_from_server(
    mut client: ResMut<RenetClient>,
    mut game_state: ResMut<GameState>,
    mut player_handles: ResMut<PlayerHandles>,
    mut game_events: EventWriter<GameEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut player_handles_query: Query<(&mut PlayerHandle, &mut ComponentPosition)>,
) {
    let texture_handle_self = asset_server.load("sprites/bob.png");
    let texture_handle_others = asset_server.load("sprites/fritz.png");
    let texture_atlas_self = TextureAtlas::from_grid(texture_handle_self, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_others = TextureAtlas::from_grid(texture_handle_others, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_handle_self = texture_atlases.add(texture_atlas_self);
    let texture_atlas_handle_others = texture_atlases.add(texture_atlas_others);

    while let Some(message) = client.receive_message(0) {
        // Whenever the server sends a message we know that it must be a game event
        let event: GameEvent = bincode::deserialize(&message).unwrap();
        trace!("{:#?}", event);

        // We trust the server - It's always been good to us!
        // No need to validate the events it is sending us
        game_state.consume(&event);

        match &event {
            GameEvent::PlayerJoined { name, pos, player_id } => {
                let is_player = *player_id == client.client_id();
                let texture_atlas_handle = if is_player {
                    texture_atlas_handle_self.clone()
                } else {
                    texture_atlas_handle_others.clone()
                };

                let entity_id = commands
                    .spawn_bundle(SpriteSheetBundle {
                        texture_atlas: texture_atlas_handle.clone(),
                        sprite: TextureAtlasSprite::new(0),
                        ..Default::default()
                    })
                    .insert(Name { name: name.clone() })
                    .insert(ComponentPosition { pos: *pos })
                    .id();

                let player_handle = PlayerHandle {
                    client_id: *player_id,
                    entity: entity_id,
                    dir: Direction::Idle,
                };
                commands.entity(entity_id).insert(player_handle);

                if is_player {
                    commands.entity(entity_id).insert(Player);
                }

                player_handles.handles.insert(*player_id, player_handle);
            }
            GameEvent::PlayerDisconnected { player_id } => {
                println!("Trying to despawn Entity: {:?}", player_id);
                let player_handler_option = player_handles.handles.get(player_id);
                if let Some(player_handler) = player_handler_option {
                    println!("Despawning entity: {:?}", player_id);
                    commands.entity(player_handler.entity).despawn();
                    player_handles.handles.remove(player_id);
                } else {
                    println!("Entity not found: {:?}", player_id);
                }
            }
            GameEvent::PlayerGotKilled { .. } => {}
            GameEvent::BeginGame => {}
            GameEvent::EndGame { .. } => {}
            GameEvent::MovementKeyPressed { player_id, direction, start_pos } => {
                for (mut player_handle, mut pos) in player_handles_query.iter_mut() {
                    if player_handle.client_id == *player_id {
                        player_handle.dir = *direction;
                        pos.pos = *start_pos;
                    }
                }
            }
            GameEvent::MovementKeyReleased { player_id, position } => {
                for (mut player_handle, mut pos) in player_handles_query.iter_mut() {
                    if player_handle.client_id == *player_id {
                        player_handle.dir = Direction::Idle;
                        pos.pos = *position;
                    }
                }
            }
        }

        // Send the event into the bevy event system so systems can react to it
        game_events.send(event);
    }
}

// If there's any error network we just panic 🤷‍♂️
fn handle_renet_error(mut renet_error: EventReader<RenetError>) {
    for err in renet_error.iter() {
        panic!("PANIC ERROR: {}", err);
    }
}