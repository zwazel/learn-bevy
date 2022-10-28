use bevy::ecs::query::WorldQuery;
use bevy::prelude::Component;
use crate::PlayerId;

#[derive(Component)]
pub struct Target(pub PlayerId);

#[derive(Component)]
pub struct MoveTarget(f32, f32);

#[derive(Component)]
pub struct Unit;

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct OtherPlayerControlled(pub PlayerId);

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Neutral;

#[derive(Component)]
pub struct Friendly;