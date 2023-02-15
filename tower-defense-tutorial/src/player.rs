use std::default::Default;

use bevy::app::App;
use bevy::prelude::*;
use bevy::utils::default;
use bevy_mod_picking::PickingCameraBundle;
use leafwing_input_manager::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Player>()
            // This plugin maps inputs to an input-type agnostic action-state
            // We need to provide it with an enum which stores the possible actions a player could take
            .add_plugin(InputManagerPlugin::<PlayerMovementAction>::default())
            .add_startup_system(spawn_player)
            .add_system(camera_controls);
    }
}

// This is the list of "things in the game I want to be able to do based on input"
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum PlayerMovementAction {
    MoveForward,
    MoveBackwards,
    MoveLeft,
    MoveRight,
    RotateLeft,
    RotateRight,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Player {
    username: String,
}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    #[bundle]
    input_manager: InputManagerBundle<PlayerMovementAction>,
}

impl PlayerBundle {
    fn input_map() -> InputMap<PlayerMovementAction> {
        InputMap::new([
            (KeyCode::W, PlayerMovementAction::MoveForward),
            (KeyCode::S, PlayerMovementAction::MoveBackwards),
            (KeyCode::D, PlayerMovementAction::MoveRight),
            (KeyCode::A, PlayerMovementAction::MoveLeft),
            (KeyCode::Q, PlayerMovementAction::RotateLeft),
            (KeyCode::E, PlayerMovementAction::RotateRight),
        ])
    }
}

fn spawn_player(mut commands: Commands) {
    commands.
        spawn(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(PickingCameraBundle::default())
        .insert(PlayerBundle {
            player: Player {
                username: "Player1".to_string()
            },
            input_manager: InputManagerBundle {
                input_map: PlayerBundle::input_map(),
                ..default()
            },
        });
}

fn camera_controls(
    action_state: Query<&ActionState<PlayerMovementAction>, With<Player>>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    let action_state = action_state.single();
    let mut camera = camera_query.single_mut();

    let mut forward = camera.forward();
    forward.y = 0.0; // camera is angled down a bit
    forward = forward.normalize(); // always constant height off ground

    let mut left = camera.left();
    left.y = 0.0;
    left = left.normalize();

    let speed = 3.0;
    let rotate_speed = 0.3;

    if action_state.pressed(PlayerMovementAction::MoveForward) {
        camera.translation += forward * time.delta_seconds() * speed;
    }
    if action_state.pressed(PlayerMovementAction::MoveBackwards) {
        camera.translation -= forward * time.delta_seconds() * speed;
    }
    if action_state.pressed(PlayerMovementAction::MoveLeft) {
        camera.translation += left * time.delta_seconds() * speed;
    }
    if action_state.pressed(PlayerMovementAction::MoveRight) {
        camera.translation -= left * time.delta_seconds() * speed;
    }
    if action_state.pressed(PlayerMovementAction::RotateLeft) {
        camera.rotate_axis(Vec3::Y, rotate_speed * time.delta_seconds())
    }
    if action_state.pressed(PlayerMovementAction::RotateRight) {
        camera.rotate_axis(Vec3::Y, -rotate_speed * time.delta_seconds())
    }
}
