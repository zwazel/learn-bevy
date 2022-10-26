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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerCommandsList(pub Vec<(PlayerId, Vec<PlayerCommand>)>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandsToSync(pub Vec<PlayerCommand>);

impl CommandsToSync {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn reset(&mut self) {
        self.0.clear();
    }
}

impl Default for CommandsToSync {
    fn default() -> Self {
        Self(Vec::new())
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

#[derive(Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct SyncedPlayerCommandsList(pub BTreeMap<Tick, SyncedPlayerCommand>);

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerSyncedPlayerCommandsList(pub SyncedPlayerCommandsList);

impl Default for ServerSyncedPlayerCommandsList {
    fn default() -> Self {
        Self(SyncedPlayerCommandsList::default())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncedPlayerCommand(pub PlayerCommandsList, pub MyDateTime);

impl Default for SyncedPlayerCommand {
    fn default() -> Self {
        Self(PlayerCommandsList::default(), MyDateTime::now())
    }
}

impl SyncedPlayerCommandsList {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for SyncedPlayerCommandsList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (tick, synced_player_command) in &self.0 {
            write!(f, "Commands for tick {}, processed at: {}\n", tick.get(), synced_player_command.1)?;
            write!(f, "{}\n\n", synced_player_command.0)?;
        }

        Ok(())
    }
}

impl Default for SyncedPlayerCommandsList {
    fn default() -> Self {
        Self(BTreeMap::default())
    }
}