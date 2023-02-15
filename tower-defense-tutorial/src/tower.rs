use bevy::ecs::query::QueryEntityError;
use bevy::prelude::*;
use bevy_inspector_egui::InspectorOptions;
use enum_iterator::{cardinality, Sequence};
use leafwing_input_manager::prelude::*;

use crate::{*, physics::PhysicsBundle};
use crate::player::Player;
use crate::tower::TowerAction::BuildTower;

#[derive(InspectorOptions, Component, Clone, Copy, Debug, Sequence)]
pub enum TowerType {
    Tomato,
    Potato,
    Cabbage,
}

impl TowerType {
    pub fn get_image(&self, asset_server: &Res<AssetServer>) -> Handle<Image> {
        match self {
            TowerType::Tomato => {
                asset_server.load("tomato_tower.png")
            }
            TowerType::Potato => {
                asset_server.load("potato_tower.png")
            }
            TowerType::Cabbage => {
                asset_server.load("cabbage_tower.png")
            }
        }
    }
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Tower {
    pub shooting_timer: Timer,
    pub bullet_offset: Vec3,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct TowerControls;

#[derive(Bundle)]
pub struct TowerBundle {
    tower_controls: TowerControls,
    #[bundle]
    input_manager: InputManagerBundle<TowerAction>,
}

impl TowerBundle {
    fn input_map() -> InputMap<TowerAction> {
        InputMap::new([
            (KeyCode::Space, BuildTower)
        ])
    }

    pub fn spawn_tower(
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        position: Vec3,
    ) -> Entity {
        commands
            .spawn(SpatialBundle::from_transform(Transform::from_translation(
                position,
            )))
            .insert(Name::new("Tomato_Tower"))
            .insert(Tower {
                shooting_timer: Timer::from_seconds(1.5, TimerMode::Repeating),
                bullet_offset: Vec3::new(0.0, 0.6, 0.0),
            })
            .with_children(|commands| {
                commands.spawn(PbrBundle {
                    mesh: meshes.add(shape::Capsule::default().into()),
                    material: materials.add(Color::rgb(0.0, 0.84, 0.92).into()),
                    transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    ..default()
                });
            })
            .id()
    }

    pub fn spawn_tower_base(
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        position: Vec3,
    ) -> Entity {
        let default_collider_color = materials.add(Color::rgba(0.3, 0.5, 0.3, 0.3).into());
        let selected_collider_color = materials.add(Color::rgba(0.3, 0.9, 0.3, 0.9).into());

        commands.
            spawn(SpatialBundle::from_transform(Transform::from_translation(
                position
            )))
            .insert(Name::new("Tower_Base"))
            .insert(meshes.add(shape::Capsule::default().into()))
            .insert(Highlighting {
                initial: default_collider_color.clone(),
                hovered: Some(selected_collider_color.clone()),
                pressed: Some(selected_collider_color.clone()),
                selected: Some(selected_collider_color),
            })
            .insert(default_collider_color)
            .insert(NotShadowCaster)
            .insert(PickableBundle::default())
            .with_children(|commands| {
                commands.
                    spawn(PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                        material: materials.add(Color::rgb(0.67, 0.84, 0.92).into()),
                        transform: Transform::from_xyz(0.0, -0.8, 0.0),
                        ..default()
                    });
            })
            .id()
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum TowerAction {
    BuildTower,
}

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.
            register_type::<Tower>()
            .add_plugin(InputManagerPlugin::<TowerAction>::default())
            .add_startup_system_set_to_stage(
                StartupStage::PostStartup,
                SystemSet::new()
                    .with_system(setup_player_controls),
            )
            .add_startup_system(setup_player_controls)
            .add_system(tower_shooting)
            .add_system(build_tower);
    }
}

fn setup_player_controls(
    mut commands: Commands,
    mut player: Query<Entity, With<Player>>,
) {
    let mut player = player.get_single_mut();
    match player {
        Ok(player) => {
            commands.entity(player).insert(TowerBundle {
                tower_controls: TowerControls,
                input_manager: InputManagerBundle {
                    input_map: TowerBundle::input_map(),
                    ..default()
                },
            });
            println!("Set up player controls for towers");
        }
        Err(_) => {
            println!("Can't setup player controls for towers, Player can't be found.");
        }
    }
}

fn tower_shooting(
    mut commands: Commands,
    mut towers: Query<(Entity, &mut Tower, &GlobalTransform)>,
    targets: Query<&GlobalTransform, With<Target>>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (tower_ent, mut tower, transform) in &mut towers {
        tower.shooting_timer.tick(time.delta());
        if tower.shooting_timer.just_finished() {
            let bullet_spawn = transform.translation() + tower.bullet_offset;

            let direction = targets
                .iter()
                .min_by_key(|target_transform| {
                    FloatOrd(Vec3::distance(target_transform.translation(), bullet_spawn))
                })
                .map(|closest_target| closest_target.translation() - bullet_spawn);

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
                            timer: Timer::from_seconds(1000.5, TimerMode::Once),
                        })
                        .insert(Bullet {
                            direction,
                            speed: 2.5,
                        })
                        .insert(Name::new("Bullet"))
                        .insert(PhysicsBundle::moving_entity(Vec3::new(0.2, 0.2, 0.2)));
                });
            }
        }
    }
}

fn build_tower(
    mut commands: Commands,
    selection: Query<(Entity, &Selection, &Transform)>,
    action_state: Query<&ActionState<TowerAction>, With<TowerControls>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for action_state in &action_state {
        if action_state.just_pressed(BuildTower) {
            for (entity, selection, transform) in &selection {
                if selection.selected() {
                    commands.entity(entity).despawn_recursive();
                    TowerBundle::spawn_tower(&mut commands, &mut meshes, &mut materials, transform.translation);
                }
            }
        }
    }
}
