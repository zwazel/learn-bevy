use std::ops::{Deref, Mul};

use bevy::DefaultPlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_resource(PlanetsID(Vec::new()))
        .add_startup_system(setup_camera)
        .add_startup_system(setup_entities)
        .add_system(update_velocity)
        .add_system(update_position)
        .run();
}

const UNIVERSE_GRAVITATIONAL_CONSTANT: f32 = 0.0000000000667408;
const SCALE_CONST: f32 = 10.0;

#[derive(Component)]
struct Movable;

struct PlanetsID(Vec<u128>);

#[derive(Component)]
struct Planet {
    radius: f32,
    mass: f32,
    initial_velocity: Vec3,
    velocity: Vec3,
}

fn update_position(
    time: Res<Time>,
    mut planets: Query<(&Planet, &mut Transform, Option<&Movable>)>,
) {
    fn multiply_by_scalar(vec: Vec3, scalar: f32) -> Vec3 {
        vec * scalar
    }

    for (planet, mut transform, movable) in planets.iter_mut() {
        if let Some(_) = movable {
            transform.translation += planet.velocity * time.delta().as_secs_f32();
        }
    }
}

fn update_velocity(
    time: Res<Time>,
    mut planets: Query<(Entity, &mut Planet, &Transform, Option<&Movable>)>,
) {
    fn magnitude_squared(v: Vec3) -> f32 {
        v.x * v.x + v.y * v.y + v.z * v.z
    }

    let planets = planets.iter_mut();
    let other_planets = planets.;

    for (entity, mut planet, planet_transform, planet_movable) in planets {
        if let Some(_) = planet_movable {
            for (other_entity, other_planet, other_planet_transform, _) in other_planets {
                if entity.id() != other_entity.id() {
                    let distance = other_planet_transform.translation - planet_transform.translation;
                    let distance_squared = magnitude_squared(other_planet_transform.translation - planet_transform.translation);
                    let force_direction = distance.normalize();
                    let force = force_direction * UNIVERSE_GRAVITATIONAL_CONSTANT * planet.mass * other_planet.mass / distance_squared;
                    let acceleration = force / planet.mass;
                    planet.velocity += acceleration * time.delta().as_secs_f32();
                }
            }
        }
    }
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
            initial_velocity: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
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
            initial_velocity: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
        })
        .insert(Movable)
    ;
}