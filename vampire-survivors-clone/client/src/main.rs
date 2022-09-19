use std::net::UdpSocket;
use std::time::SystemTime;
use bevy::prelude::*;
use bevy_renet::{RenetClientPlugin, run_if_client_connected};
use renet::{ClientAuthentication, NETCODE_USER_DATA_BYTES, RenetClient, RenetConnectionConfig, RenetError};

use store::{GameEvent, GameState, HOST, PORT, PROTOCOL_ID};

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
        .insert_resource(ClearColor(Color::hex("282828").unwrap()))
        .add_plugins(DefaultPlugins)
        // Renet setup
        .add_plugin(RenetClientPlugin)
        .insert_resource(new_renet_client(&username).unwrap())
        .add_system(handle_renet_error)
        .add_system_to_stage(
            CoreStage::PostUpdate,
            receive_events_from_server.with_run_criteria(run_if_client_connected),
        )
        // Add our game state and register GameEvent as a bevy event
        .insert_resource(GameState::default())
        .add_event::<GameEvent>()
        // Add setup function to spawn UI and board graphics
        .add_startup_system(setup)
        // Finally we run the thing!
        .run();
}

#[derive(Component)]
struct PlayerHandle(pub u64);

////////// SETUP //////////
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());
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

fn receive_events_from_server(
    mut client: ResMut<RenetClient>,
    mut game_state: ResMut<GameState>,
    mut game_events: EventWriter<GameEvent>,
) {
    while let Some(message) = client.receive_message(0) {
        // Whenever the server sends a message we know that it must be a game event
        let event: GameEvent = bincode::deserialize(&message).unwrap();
        trace!("{:#?}", event);

        // We trust the server - It's always been good to us!
        // No need to validate the events it is sending us
        game_state.consume(&event);

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