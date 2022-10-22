use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use std::time::{Instant, SystemTime};

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

#[derive(Deserialize)]
pub struct PlayerCommandsList(pub Vec<(PlayerId, Vec<PlayerCommand>)>);

impl Serialize for PlayerCommandsList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (player_id, commands) in &self.0 {
            map.serialize_entry(&player_id.0, &commands)?;
        }
        map.end()
    }
}

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

pub struct MyDateTime(pub DateTime<Local>);

impl MyDateTime {
    pub fn now() -> Self {
        Self(Local::now())
    }

    pub fn to_string(&self) -> String {
        self.0.format("%d-%m-%Y_%H-%M-%S").to_string()
    }
}

impl Display for MyDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_rfc2822())
    }
}

impl Serialize for MyDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&self.0.to_rfc2822())
    }
}

impl<'de> Deserialize<'de> for MyDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(DateTime::from(DateTime::parse_from_rfc2822(&s).unwrap())))
    }
}

#[derive(Serialize, Deserialize)]
pub struct SyncedPlayerCommandsList(pub BTreeMap<Tick, (PlayerCommandsList, MyDateTime)>);

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