use bevy::prelude::*;
use bevy_renet::{RenetClientPlugin, run_if_client_connected};

use store::{GameEvent, GameState};

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
        // Add systems for playing TicTacTussle
        .add_system(change_ui_by_stage)
        .add_system(update_waiting_text)
        .add_system(update_in_game_ui)
        .add_system(update_board)
        .add_system(input)
        // Finally we run the thing!
        .run();
}

////////// COMPONENTS //////////
#[derive(Component)]
struct UIRoot;

#[derive(Component)]
struct WaitingText;

#[derive(Component)]
struct PlayerHandle(pub u64);

////////// SETUP //////////
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    // Spawn board background
    commands.spawn_bundle(SpriteBundle {
        transform: Transform::from_xyz(0.0, -30.0, 0.0),
        sprite: Sprite {
            custom_size: Some(Vec2::new(480.0, 480.0)),
            ..default()
        },
        texture: asset_server.load("background.jpg").into(),
        ..default()
    });

    // Spawn pregame ui
    commands
        // A container that centers its children on the screen
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                size: Size::new(Val::Percent(100.0), Val::Px(60.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            color: Color::NONE.into(),
            ..default()
        })
        .insert(UIRoot)
        .with_children(|parent| {
            parent
                .spawn_bundle(TextBundle::from_section(
                    "Waiting for an opponent...",
                    TextStyle {
                        font: asset_server.load("Inconsolata.ttf"),
                        font_size: 24.0,
                        color: Color::hex("ebdbb2").unwrap(),
                    },
                ))
                .insert(WaitingText);
        });
}