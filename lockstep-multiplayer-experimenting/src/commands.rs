use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use bevy::render::render_resource::MapMode;

use serde::{Deserialize, Serialize};

use crate::{Player, PlayerId, Tick};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerCommand {
    Test(String),
}

impl Display for PlayerCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Test(s) => write!(f, "Test({})", s),
        }
    }
}

pub struct PlayerCommandsList(pub Vec<(PlayerId, Vec<PlayerCommand>)>);

impl Display for PlayerCommandsList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (player, command) in &self.0 {
            write!(f, "Commands for player {}:\n", player.0)?;

            if command.is_empty() {
                write!(f, " -\tNone (empty)\n")?;
            } else {
                for command in command {
                    write!(f, " -\t{}\n", command)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for PlayerCommandsList {
    fn default() -> Self {
        Self(Vec::new())
    }
}

pub struct SyncedPlayerCommandsList(pub BTreeMap<Tick, PlayerCommandsList>);

impl Display for SyncedPlayerCommandsList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (tick, commands) in &self.0 {
            write!(f, "Commands for tick {}:\n", tick.get())?;
            write!(f, "{}\n\n", commands)?;
        }

        Ok(())
    }
}

impl Default for SyncedPlayerCommandsList {
    fn default() -> Self {
        Self(BTreeMap::default())
    }
}