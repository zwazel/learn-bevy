#![feature(core_intrinsics)]

use core::default::Default;
use std::intrinsics::floorf32;
use std::ops::Mul;

use bevy::prelude::*;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .insert_resource(Global {
            grid_size: Vec2::new(500.0, 500.0),
            scale: 10.0,
        })
        .insert_resource(MoveTimer {
            timer: Timer::from_seconds(0.1, true),
        })
        .add_system_set(SystemSet::new()
            .with_system(snake_movement))
        .run();
}

struct Global {
    grid_size: Vec2,
    scale: f32,
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

#[derive(Component)]
struct Food;

fn setup(mut commands: Commands, global_settings: Res<Global>) {
    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    // setup the snake
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(global_settings.scale, global_settings.scale, 0.0),
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
            translation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(global_settings.scale, global_settings.scale, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: Color::rgb(1.0, 0.5, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(SnakePiece);

    // setup the food with a random position
    let food_pos = get_random_food_pos(&global_settings);

    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(food_pos.x, food_pos.y, food_pos.z),
            scale: Vec3::new(global_settings.scale, global_settings.scale, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: Color::rgb(1.0, 0.5, 0.5),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Food);

    // add the walls
    let wall_color = Color::rgb(0.8, 0.8, 0.8);

    // left
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(-global_settings.grid_size.x / 2.0, 0.0, 0.0),
            scale: Vec3::new(global_settings.scale, global_settings.grid_size.y + global_settings.scale, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: wall_color,
            ..Default::default()
        },
        ..Default::default()
    });

    // right
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(global_settings.grid_size.x / 2.0, 0.0, 0.0),
            scale: Vec3::new(global_settings.scale, global_settings.grid_size.y + global_settings.scale, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: wall_color,
            ..Default::default()
        },
        ..Default::default()
    });

    // top
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(0.0, global_settings.grid_size.y / 2.0, 0.0),
            scale: Vec3::new(global_settings.grid_size.x + global_settings.scale, global_settings.scale, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: wall_color,
            ..Default::default()
        },
        ..Default::default()
    });

    // bottom
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            translation: Vec3::new(0.0, -global_settings.grid_size.y / 2.0, 0.0),
            scale: Vec3::new(global_settings.grid_size.x + global_settings.scale, global_settings.scale, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: wall_color,
            ..Default::default()
        },
        ..Default::default()
    });
}

fn get_random_food_pos(global_settings: &Global) -> Vec3 {
    let mut rng = rand::thread_rng();
    let food_pos = Vec3::new(
        unsafe { floorf32(rng.gen_range(0.0..((global_settings.grid_size.x / 2.0) / global_settings.scale))) },
        unsafe { floorf32(rng.gen_range(0.0..((global_settings.grid_size.y / 2.0) / global_settings.scale))) },
        0.0,
    );

    food_pos.mul(global_settings.scale)
}

fn snake_movement(keyboard_input: Res<Input<KeyCode>>, global_settings: Res<Global>, time: Res<Time>, mut move_timer: ResMut<MoveTimer>, mut query: Query<(&mut Direction, &mut Transform), (With<SnakeHead>, Without<Food>)>, mut query_food: Query<&mut Transform, (With<Food>, Without<SnakeHead>, Without<SnakePiece>)>) {
    let (mut direction, mut transform) = query.single_mut(); // this panics if there are multiple query results!
    let mut food_transform = query_food.single_mut();

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
        let translation = &mut transform.translation;
        translation.x += direction.dir.x * global_settings.scale;
        translation.y += direction.dir.y * global_settings.scale;
        println!("translation: {:?}", translation);

        // wrap around the screen
        if (translation.x + global_settings.scale) > global_settings.grid_size.x / 2.0 {
            translation.x = (-global_settings.grid_size.x / 2.0) + global_settings.scale;
            println!("wrapped right to left");
        } else if (translation.x - global_settings.scale) < -global_settings.grid_size.x / 2.0 {
            translation.x = (global_settings.grid_size.x / 2.0) - global_settings.scale;
            println!("wrapped left to right");
        } else if (translation.y + global_settings.scale) > global_settings.grid_size.y / 2.0 {
            translation.y = (-global_settings.grid_size.y / 2.0) + global_settings.scale;
            println!("wrapped up to down");
        } else if (translation.y - global_settings.scale) < -global_settings.grid_size.y / 2.0 {
            translation.y = (global_settings.grid_size.y / 2.0) - global_settings.scale;
            println!("wrapped down to up");
        }

        // check if the snake has eaten the food
        if translation.x == food_transform.translation.x && translation.y == food_transform.translation.y {
            // change position of food
            food_transform.translation = get_random_food_pos(&global_settings);
        }
    }
}