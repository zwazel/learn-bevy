use core::default::Default;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(where_are_you)
        .run();
}

fn where_are_you(query: Query<&Position, With<SnakePiece>>) {
    for snake_pos in query.iter() {
        println!("Pos: [x:{},y:{}]", snake_pos.x, snake_pos.y)
    }
}

#[derive(Component)]
struct SnakePiece;

#[derive(Component)]
struct Position {
    x: i32,
    y: i32,
}

fn setup(mut commands: Commands) {
    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    // setup the snake
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(10.0, 10.0, 0.0),
            scale: Vec3::new(10.0, 10.0, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: Color::rgb(0.5, 0.5, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(SnakePiece)
        .insert(Position {
            x: 10,
            y: 10,
        });
}