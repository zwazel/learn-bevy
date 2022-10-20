use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use std::time::Instant;

use bevy::render::render_resource::MapMode;
use chrono::{DateTime, FixedOffset, Local, Utc};
use env_logger::fmt::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeMap;

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

#[derive(Serialize, Deserialize)]
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

pub struct SyncedPlayerCommandsList(pub BTreeMap<Tick, (PlayerCommandsList, DateTime<Utc>)>);

impl Deserialize for SyncedPlayerCommandsList {
    fn deserialize<'de, D>(deserializer: D) -> Result<Self, dyn serde::de::Error> where D: Deserializer<'de> {
        let map: HashMap<Tick, (PlayerCommandsList, DateTime<Utc>)> = HashMap::deserialize(deserializer)?;

        Ok(Self(map.into_iter().collect()))
    }
}

impl Serialize for SyncedPlayerCommandsList {
    fn serialize<S>(&self, serializer: S) -> Result<serde::ser::Ok, dyn serde::ser::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (tick, (commands, timestamp)) in &self.0 {
            map.serialize_entry(&tick.0, &(commands, timestamp.to_rfc3339()))?;
        }

        map.end()
    }
}



impl Display for SyncedPlayerCommandsList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (tick, (commands, time)) in &self.0 {
            write!(f, "Commands for tick {}, processed at: {}\n", tick.get(), time)?;
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