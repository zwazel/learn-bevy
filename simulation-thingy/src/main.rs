use bevy::app::App;
use bevy::prelude::*;
use bevy::render::render_resource::std140::AsStd140;

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

#[derive(Component)]
struct Name(String);

#[derive(SystemLabel, Clone, Hash, Debug, PartialEq, Eq)]
enum DebugOrNot {
    Debug,
    NotDebug,
}

fn setup(mut commands: Commands) {
    commands.spawn().insert(Hunger(0.0)).insert(Name("Fritz".to_string()));
    commands.spawn().insert(Hunger(0.5)).insert(Name("Hans".to_string()));
}

fn hunger_system(mut commands: Commands, mut query: Query<(&mut Hunger, &Name)>) {
    // Loop through entities and increase hunger

    for (mut hunger, name) in query.iter_mut() {
        hunger.0 += 0.1;

        if hunger.0 >= 1.0 {
            hunger.0 = 0.0;
            println!("{} is hungry", name.0);
            // commands.entity(entity).despawn();
        }
    }
}

fn print_hunger_status_system(query: Query<(&Hunger, &Name)>) {
    for (hunger, name) in query.iter() {
        println!("Hunger of {}: {}", name.0, hunger.0);
    }
}