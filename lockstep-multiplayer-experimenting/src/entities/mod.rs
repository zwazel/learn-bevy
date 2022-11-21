
use bevy::prelude::Component;

use crate::PlayerId;

#[derive(Component)]
pub struct Target;

#[derive(Component)]
pub struct MoveTarget(pub f32, pub f32);

#[derive(Component)]
pub struct Unit;

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct OtherPlayerControlled(pub PlayerId);

#[derive(Component)]
pub struct OtherPlayerCamera(pub PlayerId);

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Neutral;

#[derive(Component)]
pub struct Friendly;
