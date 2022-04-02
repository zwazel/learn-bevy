use core::default::Default;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_set(SystemSet::new()
            .with_system(snake_movement))
        .run();
}

#[derive(Component)]
struct SnakePiece;

#[derive(Component)]
struct SnakeHead;

fn setup(mut commands: Commands) {
    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    let pos = (10.0, 10.0);
    // setup the snake
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(pos.0, pos.1, 0.0),
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
        .insert(SnakeHead);
}

fn snake_movement(keyboard_input: Res<Input<KeyCode>>, mut query: Query<(&SnakePiece, With<SnakeHead>, &mut Transform)>) {
    let (snake_head, _, mut transform) = query.single_mut();

    let mut direction = Vec2::new(0.0, 0.0);
    if keyboard_input.pressed(KeyCode::Left) || keyboard_input.pressed(KeyCode::A) {
        direction.x -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::Right) || keyboard_input.pressed(KeyCode::D) {
        direction.x += 1.0;
    }

    if keyboard_input.pressed(KeyCode::Up) || keyboard_input.pressed(KeyCode::W) {
        direction.y += 1.0;
    }

    if keyboard_input.pressed(KeyCode::Down) || keyboard_input.pressed(KeyCode::S) {
        direction.y -= 1.0;
    }

    let translation = &mut transform.translation;
    translation.x += direction.x;
    translation.y += direction.y;
}