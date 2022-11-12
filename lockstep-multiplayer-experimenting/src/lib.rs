extern crate core;

use std::cmp::*;
use std::collections::*;
use std::fmt::*;
use std::time::*;

use bevy::math::Vec3;
use bevy::prelude::{Component, Entity, Vec2};
use renet::{ChannelConfig, NETCODE_KEY_BYTES, ReliableChannelConfig, RenetConnectionConfig, UnreliableChannelConfig};
use serde::{Deserialize, Serialize};

use crate::commands::{MyDateTime, PlayerCommand, PlayerCommandsList, SyncedPlayerCommand};

pub mod commands;
pub mod server_functionality;
pub mod client_functionality;
pub mod asset_handling;
pub mod entities;

pub const PORT: i32 = 5000;
pub const AMOUNT_PLAYERS: usize = 4;

pub const PRIVATE_KEY: &[u8; NETCODE_KEY_BYTES] = b"an example very very secret key.";
pub const PROTOCOL_ID: u64 = 6969;

pub const TICKRATE: u64 = 250;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ServerTick(pub Tick);

pub struct ServerMarker;

impl ServerTick {
    pub fn new() -> Self {
        Self(Tick::new())
    }

    pub fn get(&self) -> i64 {
        self.0.get()
    }

    pub fn set(&mut self, tick: i64) {
        self.0.set(tick);
    }

    pub fn increment(&mut self) {
        self.0.increment();
    }

    pub fn reset(&mut self) {
        self.0.reset();
    }
}

#[derive(Debug)]
pub enum Speeds {
    Normal(Vec3),
    Sprint(Vec3),
}

impl Speeds {
    pub fn get(&self) -> Vec3 {
        match self {
            Self::Normal(v) => *v,
            Self::Sprint(v) => *v,
        }
    }
}

pub enum DefaultSpeeds {
    Normal,
    Sprint,
}

impl DefaultSpeeds {
    pub fn get(&self) -> Speeds {
        match self {
            DefaultSpeeds::Normal => Speeds::Normal(Vec3::new(10.0, 10.0, 10.0)),
            DefaultSpeeds::Sprint => Speeds::Sprint(Vec3::new(20.0, 20.0, 20.0)),
        }
    }
}

#[derive(Debug, Component)]
pub struct CameraMovement {
    // x = left/right y = up/down z = forward/backward
    pub velocity: Vec3,
    pub acceleration: f32,
    pub deceleration: f32,
    pub skid_deceleration: f32,
    pub max_speed: Speeds,
    pub mouse_sensitivity: f32,
    pub last_mouse_position: Vec2,

    pub scroll_speed: f32,
    pub scroll_acceleration: f32,
    pub scroll_deceleration: f32,
    pub target_camera_height: f32,
    pub scroll_error_tolerance: f32,
}

impl Default for CameraMovement {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            acceleration: 2.0,
            deceleration: 0.1,
            skid_deceleration: 3.0,
            max_speed: DefaultSpeeds::Normal.get(),
            mouse_sensitivity: 1.0,

            last_mouse_position: Default::default(),
            scroll_speed: 1.0,
            scroll_acceleration: 3.0,
            scroll_deceleration: 0.1,
            target_camera_height: 0.0,
            scroll_error_tolerance: 0.01,
        }
    }
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    Loading,
    InGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Component, PartialOrd)]
pub struct Tick(pub i64);

impl Ord for Tick {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Tick {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn get(&self) -> i64 {
        self.0
    }

    pub fn set(&mut self, tick: i64) {
        self.0 = tick;
    }

    pub fn increment(&mut self) {
        self.0 = self.get() + 1;
    }

    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Component)]
pub struct PlayerId(pub u64);

impl Display for PlayerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

// Clients last received ticks
#[derive(Debug, Default)]
pub struct ClientTicks(pub HashMap<PlayerId, Tick>);

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub username: Username,
    pub entity: Option<Entity>,
}

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct Username(pub String);

impl Display for Username {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

impl Player {
    pub fn default_username() -> Username {
        Username(format!("Player_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()))
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: PlayerId(0),
            username: Self::default_username(),
            entity: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct ServerLobby(pub HashMap<PlayerId, Player>);

impl ServerLobby {
    pub fn get_username(&self, player_id: PlayerId) -> Option<String> {
        self.0.get(&player_id).map(|player| player.username.0.clone())
    }
}

pub struct ClientLobby(pub HashMap<PlayerId, PlayerInfo>);

impl Default for ClientLobby {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl ClientLobby {
    pub fn get_username(&self, player_id: PlayerId) -> Option<String> {
        self.0.get(&player_id).map(|player| player.username.0.clone())
    }
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub client_entity: Entity,
    pub server_entity: Entity,
    pub username: Username,
}

pub enum ClientType {
    Client,
    Server,
}

impl Debug for ClientType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ClientType::Client => write!(f, "Client"),
            ClientType::Server => write!(f, "Server"),
        }
    }
}

impl Display for ClientType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ClientType::Client => write!(f, "Client"),
            ClientType::Server => write!(f, "Server"),
        }
    }
}

#[derive(Default)]
pub struct NetworkMapping(HashMap<Entity, Entity>);

pub enum ClientChannel {
    Input,
    ClientTick,
}

pub enum ServerChannel {
    ServerMessages,
    ServerTick,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    PlayerCreate { entity: Entity, player: Player },
    PlayerRemove { id: PlayerId },
    UpdateTick {
        target_tick: Tick,
        commands: SyncedPlayerCommand,
    },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientMessages {
    ClientUpdateTick {
        current_tick: Tick,
        commands: Vec<PlayerCommand>,
    },
}

impl ClientChannel {
    pub fn id(&self) -> u8 {
        match self {
            Self::Input => 1,
            Self::ClientTick => 2
        }
    }

    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ReliableChannelConfig {
                channel_id: Self::Input.id(),
                message_resend_time: Duration::ZERO,
                ..Default::default()
            }
                .into(),
            ReliableChannelConfig {
                channel_id: Self::ClientTick.id(),
                message_resend_time: Duration::ZERO,
                ..Default::default()
            }
                .into(),
        ]
    }
}

impl ServerChannel {
    pub fn id(&self) -> u8 {
        match self {
            Self::ServerMessages => 3,
            Self::ServerTick => 4,
        }
    }

    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ReliableChannelConfig {
                channel_id: Self::ServerMessages.id(),
                message_resend_time: Duration::from_millis(200),
                ..Default::default()
            }
                .into(),
            ReliableChannelConfig {
                channel_id: Self::ServerTick.id(),
                message_resend_time: Duration::from_millis(50),
                ..Default::default()
            }
                .into(),
        ]
    }
}

pub fn client_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ClientChannel::channels_config(),
        receive_channels_config: ServerChannel::channels_config(),
        ..Default::default()
    }
}

pub fn server_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ServerChannel::channels_config(),
        receive_channels_config: ClientChannel::channels_config(),
        ..Default::default()
    }
}

pub fn translate_port(port: &str) -> i32 {
    port.parse::<i32>().unwrap_or(PORT)
}

pub fn translate_host<'a>(host: &'a str, default: &'a str) -> &'a str {
    // 127.0.0.1 if default is not provided
    let default = if default.is_empty() {
        "127.0.0.1"
    } else {
        default
    };
    let host = match host {
        "localhost" => default,
        "-" => default,
        _ => host,
    };
    host
}
