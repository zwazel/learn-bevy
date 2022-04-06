use bevy::DefaultPlugins;
use bevy::prelude::{App, Commands, OrthographicCameraBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}

fn setup(mut commands: Commands) {
    // setup 2d camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}