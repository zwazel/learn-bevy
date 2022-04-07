use bevy::app::App;
use bevy::prelude::*;

const WORLD_HEIGHT: u32 = 100;
const WORLD_WIDTH: u32 = 100;

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
struct Hunger(f32, f32); // first value is hunger, second is max hunger. if max hunger is reached they die

#[derive(Component)]
struct HungerModifier(f32); // changes how quickly they get hungry. + value means faster more hunger, - value means less fast hunger

#[derive(Component)]
struct Health(f32, f32); // first value is current health, second is max health. if current health is reached they die

#[derive(Component)]
struct HealthModifier(f32);

#[derive(Component)]
struct Name(String);

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

fn setup_entities(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bob_sprite: Handle<Sprite> = asset_server.load("assets/sprites/bob.png");

    commands
        .spawn_bundle(SpriteBundle {
            sprite: bob_sprite,
            ..Default::default()
        })
        .insert(Hunger(0.0, 100.0))
        .insert(Name("Fritz".to_string()))
        .insert(Health(100.0, 100.0))
        .insert(Position { x: 10, y: 10 })
        .insert(Size::square(3.0))
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

fn hunger_check_system(mut commands: Commands, mut query: Query<(Entity, &Hunger, &Name)>) {
    for (entity, hunger, name) in query.iter_mut() {
        if hunger.0 >= hunger.1 {
            println!("{} died of hunger", name.0);
            commands.entity(entity).despawn();
        }
    }
}

fn hunger_system(mut query: Query<(&mut Hunger, &HungerModifier)>) {
    for (mut hunger, hunger_modifier) in query.iter_mut() {
        if hunger.0 + hunger_modifier.0 > 0.0 {
            hunger.0 += hunger_modifier.0;
        } else {
            hunger.0 = 0.0;
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