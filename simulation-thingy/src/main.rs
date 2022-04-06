use bevy::app::App;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_startup_system(setup)
        .add_system(hunger_system)
        .add_system(print_hunger_status_system)
        .run();
}

#[derive(Component)]
struct Hunger(f32);

fn setup(mut commands: Commands) {
    commands.spawn().insert(Hunger(0.0));
    commands.spawn().insert(Hunger(69.0));
}

fn hunger_system(mut query: Query<&mut Hunger>) {
    for hunger in query.iter_mut() {
        hunger.0 += 0.001;
    }
}

fn print_hunger_status_system(mut query: Query<&Hunger>) {
    for hunger in query.iter() {
        println!("Hunger: {}", hunger.0);
    }
}