use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const PORT: i32 = 5000;
pub const AMOUNT_PLAYERS: usize = 4;
pub const HOST: &str = "127.0.0.1";
pub const PROTOCOL_ID: u64 = 6969;

/// Struct for storing player related data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub pos: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

// This just makes it easier to discern between a player id and any ol' u64
type PlayerId = u64;

/// The different states a game can be in. (not to be confused with the entire "GameState")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stage {
    PreGame,
    InGame,
    Ended,
}

/// An event that progresses the GameGameState forward
#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub enum GameEvent {
    BeginGame,
    EndGame { reason: EndGameReason },
    PlayerJoined { player_id: PlayerId, name: String, pos: Position },
    PlayerDisconnected { player_id: PlayerId },
    PlayerGotKilled { player_id: PlayerId, killer_entity: String },
}

/// The various reasons why a game could end
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Deserialize)]
pub enum EndGameReason {
    BothPlayersDied,
    PlayerEndedTheGame { player_id: PlayerId },
    ReturningToLobby,
}

/// A GameState object that is able to keep track of the, uh, well.... state of the game
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub stage: Stage,
    pub players: HashMap<PlayerId, Player>,
    pub history: Vec<GameEvent>,
}

impl GameState {
    /// Determines whether an event is valid considering the current GameState
    pub fn validate(&self, event: &GameEvent) -> bool {
        use GameEvent::*;

        match event {
            EndGame { reason } => match reason {
                EndGameReason::BothPlayersDied { .. } => {
                    // todo
                    return false;
                }
                _ => {}
            },
            BeginGame { .. } => {
                if self.stage != Stage::PreGame {
                    return false;
                }
            }
            PlayerJoined { player_id, .. } => {
                if self.players.contains_key(player_id) {
                    return false;
                }
            }
            PlayerDisconnected { player_id } => {
                if !self.players.contains_key(player_id) {
                    return false;
                }
            }
            PlayerGotKilled { player_id, .. } => {
                if !self.players.contains_key(player_id) {
                    return false;
                }
            }
        }
        true
    }

    /// Consumes an event, modifying the GameState and adding the event to its history
    /// NOTE: consume assumes the event to have already been validated and will accept *any* event passed to it
    pub fn consume(&mut self, valid_event: &GameEvent) {
        use GameEvent::*;
        match valid_event {
            BeginGame { .. } => {
                self.stage = Stage::InGame;
            }
            EndGame { reason: _ } => self.stage = Stage::Ended,
            PlayerJoined { player_id, name, pos } => {
                self.players.insert(
                    *player_id,
                    Player {
                        name: name.to_string(),
                        pos: *pos,
                    },
                );

                println!("Player {} joined the game at [x:{}, y:{}]", name, pos.x, pos.y);
            }
            PlayerDisconnected { player_id } => {
                self.players.remove(player_id);
                println!("Client {} disconnected", player_id);
            }
            PlayerGotKilled { player_id, killer_entity } => {
                let player = self.players.get(player_id).unwrap().name.to_string();
                println!("Player {} got killed by {}", player, killer_entity);
            }
        }

        self.history.push(valid_event.clone());
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            stage: Stage::PreGame,
            players: HashMap::new(),
            history: Vec::new(),
        }
    }
}