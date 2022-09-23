use std::net::UdpSocket;
use std::time::SystemTime;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget::Window;
use bevy::window::{WindowClosed, WindowCloseRequested, WindowPlugin, WindowSettings};
use bevy_renet::{RenetClientPlugin, run_if_client_connected};
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetConnectionConfig, RenetError};

use store::{GameEvent, GameState, HOST, PORT, Position, PROTOCOL_ID};

fn main() {
    // Get username from stdin args
    let args = std::env::args().collect::<Vec<String>>();
    let username = &args[1];

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
        .add_system(handle_renet_error
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
#[derive(Component)]
struct PlayerHandle(pub u64);

#[derive(Component, Clone, Copy, PartialEq)]
struct ComponentPosition {
    pos: Position,
}

#[derive(Component)]
struct Name {
    name: String,
}

fn position_translation(windows: Res<Windows>, mut q: Query<
    (&ComponentPosition, &mut Transform),
>) {
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
    mut game_events: EventWriter<GameEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load("sprites/bob.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    while let Some(message) = client.receive_message(0) {
        // Whenever the server sends a message we know that it must be a game event
        let event: GameEvent = bincode::deserialize(&message).unwrap();
        trace!("{:#?}", event);

        // We trust the server - It's always been good to us!
        // No need to validate the events it is sending us
        game_state.consume(&event);

        match &event {
            GameEvent::PlayerJoined { name, pos, .. } => {
                commands
                    .spawn_bundle(SpriteSheetBundle {
                        texture_atlas: texture_atlas_handle.clone(),
                        sprite: TextureAtlasSprite::new(0),
                        ..Default::default()
                    })
                    .insert(Name { name: name.clone() })
                    .insert(ComponentPosition { pos: *pos })
                ;
            }
            GameEvent::PlayerDisconnected { .. } => {}
            GameEvent::PlayerGotKilled { .. } => {}
            _ => {}
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