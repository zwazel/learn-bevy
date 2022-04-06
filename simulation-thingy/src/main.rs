use bevy::app::App;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(hunger_system
            .label(DebugOrNot::NotDebug)
            .before(DebugOrNot::Debug)
        )
        .add_system(print_hunger_status_system
            .label(DebugOrNot::Debug)
            .after(DebugOrNot::NotDebug)
        )
        .run();
}

#[derive(Component)]
struct Hunger(f32);

#[derive(SystemLabel, Clone, Hash, Debug, PartialEq, Eq)]
enum DebugOrNot {
    Debug,
    NotDebug,
}

fn setup(mut commands: Commands) {
    commands.spawn().insert(Hunger(0.0));
    commands.spawn().insert(Hunger(69.0));
}

fn hunger_system(mut query: Query<&mut Hunger>) {
    for mut hunger in query.iter_mut() {
        hunger.0 += 0.001;
        println!("system")
    }
}

fn print_hunger_status_system(query: Query<&Hunger>) {
    for hunger in query.iter() {
        println!("Hunger: {}", hunger.0);
    }
}