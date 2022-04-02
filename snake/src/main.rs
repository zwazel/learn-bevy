use core::default::Default;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .insert_resource(Global {
            grid_size: Vec2::new(500.0, 500.0),
            scale: Vec2::new(10.0, 10.0),
        })
        .insert_resource(MoveTimer {
            timer: Timer::from_seconds(0.2, true),
        })
        .add_system_set(SystemSet::new()
            .with_system(snake_movement))
        .run();
}

struct Global {
    grid_size: Vec2,
    scale: Vec2,
}

struct MoveTimer {
    timer: Timer,
}

#[derive(Component)]
struct SnakePiece;

#[derive(Component)]
struct SnakeHead;

#[derive(Component)]
struct Direction {
    dir: Vec2,
}

fn setup(mut commands: Commands, global_settings: Res<Global>) {
    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    // setup the snake
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(global_settings.grid_size.x / 2.0, global_settings.grid_size.y / 2.0, 0.0),
            scale: Vec3::new(global_settings.scale.x, global_settings.scale.y, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: Color::rgb(0.5, 0.5, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(SnakePiece)
        .insert(SnakeHead)
        .insert(Direction {
            dir: Vec2::new(1.0, 0.0),
        });

    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(global_settings.grid_size.x / 2.0, global_settings.grid_size.y / 2.0, 0.0),
            scale: Vec3::new(10.0, 10.0, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: Color::rgb(1.0, 0.5, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(SnakePiece);
}

fn snake_movement(keyboard_input: Res<Input<KeyCode>>, global_settings: Res<Global>, time: Res<Time>, mut move_timer: ResMut<MoveTimer>, mut query: Query<(With<SnakeHead>, &mut Direction, &mut Transform)>) {
    let (_, mut direction, mut transform) = query.single_mut(); // this panics if there are multiple query results!

    if keyboard_input.pressed(KeyCode::Left) || keyboard_input.pressed(KeyCode::A) {
        direction.dir.x = -1.0;
        direction.dir.y = 0.0;
    } else if keyboard_input.pressed(KeyCode::Right) || keyboard_input.pressed(KeyCode::D) {
        direction.dir.x = 1.0;
        direction.dir.y = 0.0;
    } else if keyboard_input.pressed(KeyCode::Up) || keyboard_input.pressed(KeyCode::W) {
        direction.dir.y = 1.0;
        direction.dir.x = 0.0;
    } else if keyboard_input.pressed(KeyCode::Down) || keyboard_input.pressed(KeyCode::S) {
        direction.dir.y = -1.0;
        direction.dir.x = 0.0;
    }

    if move_timer.timer.tick(time.delta()).just_finished() {
        println!("{:?}", transform.translation);
        let translation = &mut transform.translation;
        translation.x += direction.dir.x * global_settings.scale.x;
        translation.y += direction.dir.y * global_settings.scale.y;

        // wrap around the screen
        if (translation.x) > global_settings.grid_size.x {
            translation.x = 0.0 + global_settings.scale.x;
            println!("wrapped right to left");
        } else if translation.x < 0.0 {
            translation.x = global_settings.grid_size.x - global_settings.scale.x;
            println!("wrapped left to right");
        } else if (translation.y) > global_settings.grid_size.y {
            translation.y = 0.0 + global_settings.scale.y;
            println!("wrapped up to down");
        } else if translation.y < 0.0 {
            translation.y = global_settings.grid_size.y - global_settings.scale.y;
            println!("wrapped down to up");
        }
    }
}