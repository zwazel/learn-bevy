use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use crate::GameState;

#[derive(AssetCollection)]
pub struct TargetAssets {
    #[asset(path = "sprites/target_thingy.png")]
    pub enemy_target: Handle<Image>,
    #[asset(path = "sprites/target_thingy_friendly.png")]
    pub friendly_target: Handle<Image>,
}