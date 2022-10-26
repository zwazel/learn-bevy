use bevy::asset::Handle;
use bevy::prelude::{Image, Plugin};
use crate::GameState;

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        AssetLoader::new(GameState::Loading, GameState::Playing)
            .with_collection::<TargetAssets>()
            .build(app);
    }
}

#[derive(AssetCollection)]
pub struct TargetAssets {
    #[asset(path = "sprites/target_thingy.png")]
    pub enemy_target: Handle<Image>,
    #[asset(path = "sprites/target_thingy_friendly.png")]
    pub friendly_target: Handle<Image>,
}