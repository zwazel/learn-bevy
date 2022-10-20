use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::{Player, PlayerId, Tick};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerCommand {
    Test(String),
}

pub struct PlayerCommandsList(pub Vec<(PlayerId, Vec<PlayerCommand>)>);

impl Default for PlayerCommandsList {
    fn default() -> Self {
        Self(Vec::new())
    }
}

pub struct SyncedPlayerCommandsList(pub HashMap<Tick, PlayerCommandsList>,);

impl Default for SyncedPlayerCommandsList {
    fn default() -> Self {
        Self(HashMap::default())
    }
}