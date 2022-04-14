use bevy::DefaultPlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_entities)
        .add_system(update_velocity)
        .add_system(update_position)
        .run();
}

const UNIVERSE_GRAVITATIONAL_CONSTANT: f32 = 0.0001;
const PHYSICS_TIME_STEP: f32 = 0.01;

#[derive(Component)]
struct Movable;

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Planet {
    radius: f32,
    mass: f32,
    velocity: Vec3,
}

fn update_position(
    time: Res<Time>,
    mut planets: Query<(&Planet, &mut Transform, Option<&Movable>, Option<&Name>)>,
) {
    for (planet, mut transform, movable, opt_name) in planets.iter_mut() {
        if let Some(_) = movable {
            // planet.velocity
            println!("velocity{}: {:?}",
                     if let Some(name) = opt_name {
                         " of ".to_owned() + &name.0
                     } else {
                         "".to_string()
                     }, planet.velocity);

            transform.translation += planet.velocity * PHYSICS_TIME_STEP;

            let transform_position = transform.translation;
            println!("position{}: {:?}",
                     if let Some(name) = opt_name {
                         " of ".to_owned() + &name.0
                     } else {
                         "".to_string()
                     }, transform_position);
        }
    }
}

fn update_velocity(
    time: Res<Time>,
    mut planets: Query<(Entity, &mut Planet, &Transform, Option<&Movable>, Option<&Name>)>,
) {
    fn magnitude_squared(v: Vec3) -> f32 {
        (v.x * v.x) + (v.y * v.y) + (v.z * v.z)
    }

    let mut planets_velocities = Vec::new();

    for (entity, planet, planet_transform, planet_movable, _opt_name) in planets.iter() {
        if let Some(_) = planet_movable {
            let mut current_velocity: Vec3 = planet.velocity;
            let name = if let Some(name) = _opt_name {
                name.0.clone()
            } else {
                "".to_string()
            };

            for (other_entity, other_planet, other_planet_transform, _, _) in planets.iter() {
                if entity.id() != other_entity.id() {
                    let distance: Vec3 = other_planet_transform.translation - planet_transform.translation;
                    let distance_squared: f32 = magnitude_squared(other_planet_transform.translation - planet_transform.translation);

                    println!("distance_squared{}: {}",
                             if let Some(name) = _opt_name {
                                 " of ".to_owned() + &name.0
                             } else {
                                 "".to_string()
                             }, distance_squared);

                    let force_direction: Vec3 = distance.normalize();

                    println!("force_direction{}: {:?}",
                             if let Some(name) = _opt_name {
                                 " of ".to_owned() + &name.0
                             } else {
                                 "".to_string()
                             }, force_direction);

                    let force: Vec3 = force_direction * UNIVERSE_GRAVITATIONAL_CONSTANT * planet.mass * other_planet.mass / distance_squared;
                    let acceleration: Vec3 = force / planet.mass;
                    current_velocity += acceleration * PHYSICS_TIME_STEP;
                }
            }
            planets_velocities.push((name, entity.id(), current_velocity));
        }
    }

    let mut counter = 0;
    for (planet_entity, mut planet, _, planet_movable, _) in planets.iter_mut() {
        if let Some(_) = planet_movable {
            if let Some((name, entity_id, velocity)) = planets_velocities.get(counter) {
                if *entity_id == planet_entity.id() {
                    planet.velocity = *velocity;
                    println!("new_velocity of {}: {:?}", name, planet.velocity);
                    counter += 1;
                } else {
                    panic!("Entity ID mismatch");
                }
            } else {
                panic!("Velocity not found!")
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
            translation: Vec3::new(100.0, 100.0, 0.0),
            scale: Vec3::new(100.0, 100.0, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Planet {
            radius,
            mass: 3000.0,
            velocity: Vec3::new(0.0, 0.0, 0.0),
        })
        .insert(Movable)
        .insert(Name("Blue Planet".to_string()))
    ;

    let radius = 50.0;
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            // white planet
            color: Color::rgb(1.0, 1.0, 1.0),
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(10.0, 50.0, 0.0),
            scale: Vec3::new(50.0, 50.0, 1.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Planet {
            radius,
            mass: 100.0,
            velocity: Vec3::new(10.0, 0.0, 0.0),
        })
        .insert(Movable)
        .insert(Name("White Planet".to_string()))
    ;
}