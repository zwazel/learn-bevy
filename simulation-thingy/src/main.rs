use bevy::app::App;
use bevy::prelude::*;

const WORLD_HEIGHT: u32 = 1000;
const WORLD_WIDTH: u32 = 1000;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .insert_resource(WindowDescriptor {
            title: "Simulation!".to_string(),
            width: 500.0,
            height: 500.0,
            ..Default::default()
        })
        .add_startup_system(setup_camera)
        .add_startup_system(setup_entities)
        .add_system(hunger_system
            .label(DebugOrNot::NotDebug)
            .before(DebugOrNot::Debug)
        )
        .add_system(print_hunger_status_system
            .label(DebugOrNot::Debug)
            .after(DebugOrNot::NotDebug)
        )
        .add_system(hunger_check_system
            .label(DebugOrNot::NotDebug)
            .before(DebugOrNot::Debug)
        )
        .add_system(health_system
            .label(DebugOrNot::NotDebug)
            .before(DebugOrNot::Debug)
        )
        .add_system(health_check_system
            .label(DebugOrNot::NotDebug)
            .before(DebugOrNot::Debug)
        )
        .add_system(print_health_status_system
            .label(DebugOrNot::Debug)
            .after(DebugOrNot::NotDebug)
        )
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation)
                .with_system(size_scaling),
        )
        .add_plugins(DefaultPlugins)
        .run();
}

#[derive(Component)]
struct Hunger(f32, f32); // current hunger value, max hunger value

#[derive(Component)]
struct HungerModifier(f32); // changes how quickly they get hungry. + value means faster more hunger, - value means less fast hunger

#[derive(Component)]
struct HungerDamageActive(bool);

#[derive(Component)]
struct Health(f32, f32); // first value is current health, second is max health. if current health is reached they die

#[derive(Component)]
struct HealthModifier(f32);

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Food(f32); // how much hunger is removed when eating

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(SystemLabel, Clone, Hash, Debug, PartialEq, Eq)]
enum DebugOrNot {
    Debug,
    NotDebug,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn setup_entities(mut commands: Commands, asset_server: Res<AssetServer>, mut texture_atlases: ResMut<Assets<TextureAtlas>>) {
    let texture_handle = asset_server.load("sprites/bob.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            sprite: TextureAtlasSprite::new(0),
            ..Default::default()
        })
        .insert(Hunger(100.0, 100.0))
        .insert(HungerModifier(-1.0))
        .insert(Name("Fritz".to_string()))
        .insert(Health(100.0, 100.0))
        .insert(Position { x: 100, y: 100 })
        .insert(HungerDamageActive(false))
    ;

    commands.spawn().insert_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(1.0, 0.0, 0.0),
            ..Default::default()
        },
        ..Default::default()
    })
        .insert(Position { x: 500, y: 100 })
        .insert(Size::square(64.0))
        .insert(Food(10.0))
    ;
}

fn health_check_system(mut commands: Commands, query: Query<(Entity, &Health, &Name)>) {
    for (entity, health, name) in query.iter() {
        if health.0 <= 0.0 {
            println!("{} died of death", name.0);
            commands.entity(entity).despawn();
        }
    }
}

fn health_system(mut query: Query<(&mut Health, &HealthModifier)>) {
    for (mut health, health_modifier) in query.iter_mut() {
        if health.0 + health_modifier.0 > health.1 {
            health.0 = health.1;
        } else {
            health.0 += health_modifier.0;
        }
    }
}

fn print_health_status_system(query: Query<(&Health, &Name)>) {
    for (health, name) in query.iter() {
        println!("{} has {} health", name.0, health.0);
    }
}

fn hunger_check_system(mut commands: Commands, mut query: Query<(Entity, &Hunger, &Name, &mut HungerDamageActive, Option<&mut HealthModifier>)>) {
    let hunger_damage = -1.0;
    for (entity, hunger, name, mut hunger_damage_active, health_modifier_opt) in query.iter_mut() {
        if hunger.0 <= 0.0 {
            if !hunger_damage_active.0 {
                println!("{} now takes damage because of hunger", name.0);
                if let Some(mut health_modifier) = health_modifier_opt {
                    health_modifier.0 += hunger_damage;
                } else {
                    println!("{} has no health modifier, inserted one", name.0);
                    commands.entity(entity).insert(HealthModifier(hunger_damage));
                }
                hunger_damage_active.0 = true;
            }
        } else if hunger.0 >= hunger.1 {
            if hunger_damage_active.0 {
                if let Some(mut health_modifier) = health_modifier_opt {
                    health_modifier.0 -= hunger_damage;
                }
            }

            hunger_damage_active.0 = false;
        }
    }
}

fn hunger_system(mut query: Query<(&mut Hunger, &HungerModifier)>) {
    for (mut hunger, hunger_modifier) in query.iter_mut() {
        let sum = hunger.0 + hunger_modifier.0;
        if sum <= hunger.1 && sum >= 0.0 {
            hunger.0 += hunger_modifier.0;
        }
    }
}

fn print_hunger_status_system(query: Query<(&Hunger, &Name)>) {
    for (hunger, name) in query.iter() {
        println!("Hunger of {}: {}", name.0, hunger.0);
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Transform)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut transform) in q.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / WORLD_WIDTH as f32 * window.width() as f32,
            sprite_size.height / WORLD_HEIGHT as f32 * window.height() as f32,
            1.0,
        );
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, WORLD_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, WORLD_HEIGHT as f32),
            0.0,
        );
    }
}