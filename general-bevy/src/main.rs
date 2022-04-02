use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(add_people)
        .add_system(hello_world)
        .add_system(greet_people)
        .run();
}

fn hello_world() {
    println!("hello world!");
}

// startup system - runs only once, before everything else
fn add_people(mut commands: Commands) {
    commands.spawn().insert(Person).insert(Name("Elaina Proctor".to_string()));
    commands.spawn().insert(Person).insert(Name("Renzo Hume".to_string()));
    commands.spawn().insert(Person).insert(Name("Zayna Nieves".to_string()));
}

// this system runs on all entities with the "Person" and "Name" component
// The query can be interpreted as "iterate over every Name component for entities that also have a Person component"
fn greet_people(query: Query<&Name, With<Person>>) {
    for name in query.iter() {
        println!("hello {}!", name.0);
    }
}

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name(String);