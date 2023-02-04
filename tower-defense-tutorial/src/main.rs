use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::utils::FloatOrd;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub const HEIGHT: f32 = 720.0;
pub const WIDTH: f32 = 1280.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.2)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Bevy Tower Defense".to_string(),
                width: WIDTH,
                height: HEIGHT,
                resizable: false,
                ..Default::default()
            },
            ..Default::default()
        }))
        // inspector Setup
        .add_plugin(WorldInspectorPlugin)
        .register_type::<Tower>()
        .register_type::<Lifetime>()
        .register_type::<Target>()
        .register_type::<Health>()
        .register_type::<Bullet>()

        .add_startup_system(spawn_basic_scene)
        .add_startup_system(spawn_camera)

        .add_system(tower_shooting)
        .add_system(bullet_despawn)
        .add_system(move_bullets)
        .add_system(move_targets)

        .run();
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Tower {
    shooting_timer: Timer,
    bullet_offset: Vec3,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Lifetime {
    timer: Timer,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Target {
    speed: f32,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Health {
    value: i32,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Bullet {
    direction: Vec3,
    speed: f32,
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    })
        .insert(Name::new("PlayerCamera"));
}

fn spawn_basic_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Plane
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        })
        .insert(Name::new("Ground"));

    // Tower
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.67, 0.84, 0.92).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .insert(Tower {
            shooting_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            bullet_offset: Vec3::new(0.0, 0.2, 0.5),
        })
        .insert(Name::new("Tower"));

    // Enemy / Target
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 0.4 })),
            material: materials.add(Color::rgb(0.67, 0.84, 0.92).into()),
            transform: Transform::from_xyz(-2.0, 0.2, 1.5),
            ..default()
        })
        .insert(Name::new("Target"))
        .insert(Target {
            speed: 0.3,
        })
        .insert(Health {
            value: 3
        });
}

fn tower_shooting(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut towers: Query<(Entity, &mut Tower, &GlobalTransform)>,
    targets: Query<&GlobalTransform, With<Target>>,
    time: Res<Time>,
) {
    for (tower_ent, mut tower, transform) in towers.iter_mut() {
        tower.shooting_timer.tick(time.delta());
        if tower.shooting_timer.just_finished() {
            let bullet_spawn = transform.translation() + tower.bullet_offset;

            // return closest target transform
            let direction = targets
                .iter()
                .min_by_key(|target_transform| {
                    // FloatOrd makes floats ordable
                    FloatOrd(Vec3::distance(target_transform.translation(), bullet_spawn))
                })
                .map(|closest_target| closest_target.translation() - bullet_spawn); // Turn the globaltransform into an direction

            if let Some(direction) = direction {
                commands.entity(tower_ent).with_children(|commands| {
                    commands
                        .spawn(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
                            material: materials.add(Color::rgb(0.87, 0.44, 0.42).into()),
                            transform: Transform::from_translation(tower.bullet_offset),
                            ..default()
                        })
                        .insert(Lifetime {
                            timer: Timer::from_seconds(0.5, TimerMode::Once),
                        })
                        .insert(Bullet {
                            direction,
                            speed: 2.5,
                        })
                        .insert(Name::new("Bullet"));
                });
            }
        }
    }
}

fn bullet_despawn(
    mut commands: Commands,
    mut bullets: Query<(Entity, &mut Lifetime)>,
    time: Res<Time>,
) {
    for (entity, mut lifetime) in bullets.iter_mut() {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.just_finished() {
            commands.entity(entity).despawn_recursive()
        }
    }
}

fn move_bullets(
    mut bullets: Query<(&Bullet, &mut Transform)>,
    time: Res<Time>,
) {
    for (bullet, mut transform) in bullets.iter_mut() {
        transform.translation += bullet.direction.normalize() * bullet.speed * time.delta_seconds();
    }
}

fn move_targets(
    mut targets: Query<(&Target, &mut Transform)>,
    time: Res<Time>,
) {
    for (target, mut transform) in targets.iter_mut() {
        transform.translation.x += target.speed * time.delta_seconds();
    }
}
