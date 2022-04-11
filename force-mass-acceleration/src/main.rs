use bevy::DefaultPlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_entities)
        .run();
}


const SCALE_CONST: f32 = 10.0;

#[derive(Component)]
struct Movable;

#[derive(Component)]
struct Planet {
    radius: f32,
    mass: f32,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn setup_entities(mut commands: Commands) {
    let radius = 100.0;
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            // blue planet
            color: Color::rgb(0.0, 0.0, 1.0),
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(50.0, 100.0, 0.0),
            scale: Vec3::new(radius, radius, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Planet {
            radius,
            mass: 10.0,
        })
        .insert(Movable)
    ;

    let radius = 50.0;
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            // white planet
            color: Color::rgb(1.0, 1.0, 1.0),
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(radius, radius, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Planet {
            radius,
            mass: 1.0,
        })
        .insert(Movable)
    ;
}