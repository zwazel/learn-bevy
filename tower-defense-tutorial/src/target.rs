use bevy::prelude::*;

use crate::*;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Target {
    pub speed: f32,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Health {
    pub value: i32,
}

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Target>()
            .register_type::<Health>()
            .add_system(move_targets)
            .add_system(target_death);
    }
}

fn move_targets(
    mut targets: Query<(&Target, &mut Transform)>,
    time: Res<Time>,
) {
    for (target, mut transform) in &mut targets {
        transform.translation.x += target.speed * time.delta_seconds();
    }
}

fn target_death(
    mut commands: Commands,
    targets: Query<(Entity, &Health)>,
) {
    for (entity, health) in &targets {
        if health.value <= 0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}
