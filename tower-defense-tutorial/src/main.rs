use bevy::{prelude::*, utils::FloatOrd};
use bevy::pbr::NotShadowCaster;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mod_picking::*;
use bevy_rapier3d::{
    prelude::{NoUserData, RapierConfiguration, RapierPhysicsPlugin},
    render::RapierDebugRenderPlugin,
};

pub use bullet::*;
use physics::{PhysicsBundle, PhysicsPlugin};
pub use target::*;
pub use tower::*;

use crate::player::PlayerPlugin;

pub const HEIGHT: f32 = 720.0;
pub const WIDTH: f32 = 1280.0;

mod bullet;
mod physics;
mod target;
mod tower;
mod player;

fn main() {
    App::new()
        // Window Setup
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
        // Inspector Setup
        .add_plugin(WorldInspectorPlugin)
        // init physics
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        // mod picking
        .add_plugins(DefaultPickingPlugins)
        // Our Systems
        .add_plugin(TowerPlugin)
        .add_plugin(TargetPlugin)
        .add_plugin(BulletPlugin)
        .add_plugin(PhysicsPlugin)
        .add_plugin(PlayerPlugin)
        .add_startup_system(spawn_basic_scene)
        .add_system(what_is_selected)
        .run();
}

fn spawn_basic_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    // set gravity
    rapier_config.gravity = Vec3::ZERO;

    //spawn bundle
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 50.0 })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        })
        .insert(Name::new("Ground"));

    let default_collider_color = materials.add(Color::rgba(0.3,0.5,0.3,0.3).into());
    commands.
        spawn(SpatialBundle::from_transform(Transform::from_xyz(
            0.0,0.8,0.0,
        )))
        .insert(Name::new("Tower_Base"))
        .insert(meshes.add(shape::Capsule::default().into()))

        .insert(default_collider_color)
        .insert(NotShadowCaster)
        .insert(PickableBundle::default())
        .with_children(|commands|{
            commands.
                spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                    material: materials.add(Color::rgb(0.67, 0.84, 0.92).into()),
                    transform: Transform::from_xyz(0.0, -0.8, 0.0),
                    ..default()
                });
        });

    // commands
    //     .spawn(PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //         material: materials.add(Color::rgb(0.67, 0.84, 0.92).into()),
    //         transform: Transform::from_xyz(0.0, 0.5, 0.0),
    //         ..default()
    //     })
    //     .insert(Tower {
    //         shooting_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
    //         bullet_offset: Vec3::new(0.0, 0.2, 0.5),
    //     })
    //     .insert(Name::new("Tower"));

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.4 })),
            material: materials.add(Color::rgb(0.67, 0.84, 0.92).into()),
            transform: Transform::from_xyz(-2.0, 0.2, 1.5),
            ..default()
        })
        .insert(Target { speed: 0.3 })
        .insert(Health { value: 3 })
        .insert(Name::new("Target"))
        .insert(PhysicsBundle::moving_entity(Vec3::new(0.4, 0.4, 0.4)));

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.4 })),
            material: materials.add(Color::rgb(0.67, 0.84, 0.92).into()),
            transform: Transform::from_xyz(-4.0, 0.2, 1.5),
            ..default()
        })
        .insert(Target { speed: 0.3 })
        .insert(Health { value: 3 })
        .insert(Name::new("Target"))
        .insert(PhysicsBundle::moving_entity(Vec3::new(0.2, 0.2, 0.2)));

    commands
        .spawn(PointLightBundle {
            point_light: PointLight {
                intensity: 1500.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        })
        .insert(Name::new("Light"));
}

fn what_is_selected(selection: Query<(&Name, &Selection), Changed<Selection>>) {
    for (name, selection) in &selection {
        if selection.selected() {
            info!("{}", name);
        } else {
            info!("not selected {}", name);
        }
    }
}
