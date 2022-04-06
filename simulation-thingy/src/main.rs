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
        .add_system(health_check_system
            .label(DebugOrNot::NotDebug)
            .before(DebugOrNot::Debug)
        )
        .run();
}

#[derive(Component)]
struct Hunger(f32, f32); // first value is hunger, second is max hunger. if max hunger is reached they die

#[derive(Component)]
struct HungerModifier(f32);

#[derive(Component)]
struct Name(String);

#[derive(SystemLabel, Clone, Hash, Debug, PartialEq, Eq)]
enum DebugOrNot {
    Debug,
    NotDebug,
}

fn setup(mut commands: Commands) {
    commands.spawn().insert(Hunger(0.0, 1.0)).insert(Name("Fritz".to_string()));
    commands.spawn().insert(Hunger(0.5, 12.0)).insert(Name("Hans".to_string())).insert(HungerModifier(0.1));
}

fn health_check_system(mut commands: Commands, mut query: Query<(Entity, &Hunger, &Name)>) {
    for (entity, hunger, name) in query.iter_mut() {
        if hunger.0 >= hunger.1 {
            println!("{} died of hunger", name.0);
            commands.entity(entity).despawn();
        }
    }
}

fn hunger_system(mut query: Query<(&mut Hunger, Option<&HungerModifier>)>) {
    for (mut hunger, hunger_modifier) in query.iter_mut() {
        let mut difference = 0.001;
        match hunger_modifier {
            Some(modifier) => if difference + modifier.0 > 0.0 {
                difference += modifier.0;
            },
            None => (),
        }

        hunger.0 += difference;
    }
}

fn print_hunger_status_system(query: Query<(&Hunger, &Name)>) {
    for (hunger, name) in query.iter() {
        println!("Hunger of {}: {}", name.0, hunger.0);
    }
}