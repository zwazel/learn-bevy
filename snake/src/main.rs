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
    commands.spawn().insert(SnakePiece).insert(Position {
        x: 10,
        y: 10,
    });
}