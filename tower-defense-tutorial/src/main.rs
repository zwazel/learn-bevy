use bevy::{prelude::*, utils::FloatOrd};
use bevy::pbr::NotShadowCaster;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mod_picking::*;
use bevy_rapier3d::{
    prelude::{NoUserData, RapierConfiguration, RapierPhysicsPlugin},
    render::RapierDebugRenderPlugin,
};
use enum_iterator::all;

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
        .add_plugin(PlayerPlugin)
        .add_plugin(TowerPlugin)
        .add_plugin(TargetPlugin)
        .add_plugin(BulletPlugin)
        .add_plugin(PhysicsPlugin)
        .add_startup_system(spawn_basic_scene)
        .add_startup_system(create_ui)
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

    TowerBundle::spawn_tower_base(&mut commands, &mut meshes, &mut materials, Vec3::new(0.0, 0.8, 0.0));
    TowerBundle::spawn_tower_base(&mut commands, &mut meshes, &mut materials, Vec3::new(3.0, 0.8, 0.0));

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

#[derive(Component)]
pub struct TowerUIRoot;

fn create_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mut button_icons = vec![];
    for tower_type in all::<TowerType>().collect::<Vec<_>>() {
        button_icons.push(tower_type.get_image(&asset_server));
    };

    commands.
        spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            background_color: Color::NONE.into(),
            ..default()
        })
        .insert(TowerUIRoot)
        .with_children(|commands| {
            let mut counter = 0;
            for tower_type in all::<TowerType>().collect::<Vec<_>>() {
                commands
                    .spawn(ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Percent(15.0 * 9.0 / 16.0), Val::Percent(15.0)),
                            align_self: AlignSelf::FlexStart, // Align to the bottom
                            margin: UiRect::all(Val::Percent(2.0)),
                            ..default()
                        },
                        image: button_icons[counter].clone().into(),
                        ..default()
                    })
                    .insert(tower_type);

                counter += 1;
            }
        });
}
