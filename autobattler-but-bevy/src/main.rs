use bevy::DefaultPlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

#[derive(Component)]
struct Health(i64);



fn setup(mut commands: Commands) {
    // setup 2d camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

