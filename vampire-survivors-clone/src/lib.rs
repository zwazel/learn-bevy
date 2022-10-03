use std::fmt::Debug;
use std::time::Duration;

use bevy::prelude::*;
use bevy_renet::renet::{ChannelConfig, NETCODE_KEY_BYTES, ReliableChannelConfig, RenetConnectionConfig, UnreliableChannelConfig};
use serde::{Deserialize, Serialize};

pub const PORT: i32 = 5000;
pub const AMOUNT_PLAYERS: usize = 4;

pub const PRIVATE_KEY: &[u8; NETCODE_KEY_BYTES] = b"an example very very secret key.";
pub const PROTOCOL_ID: u64 = 6969;

/// Struct for storing player related data.
#[derive(Debug, Component)]
pub struct Player {
    pub id: u64,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Component)]
pub struct PlayerInput {
    pub most_recent_tick: Option<u32>,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Component)]
pub struct Velocity(pub Vec2);

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Component)]
pub struct MaxSpeed(pub f32);

pub const PLAYER_SPEED: f32 = 100.0;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum PlayerCommand {
    BasicAttack { cast_at: Vec3 },
}

pub enum ClientChannel {
    Input,
    Command,
}

pub enum ServerChannel {
    ServerMessages,
    NetworkFrame,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    PlayerCreate { entity: Entity, id: u64, translation: [f32; 2] },
    PlayerRemove { id: u64 },
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
    pub entities: Vec<Entity>,
    pub translations: Vec<[f32; 3]>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkFrame {
    pub tick: u32,
    pub entities: NetworkedEntities,
}

impl ClientChannel {
    pub fn id(&self) -> u8 {
        match self {
            Self::Input => 1,
            Self::Command => 2,
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
                channel_id: Self::Command.id(),
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
            Self::NetworkFrame => 3,
            Self::ServerMessages => 4,
        }
    }

    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ReliableChannelConfig {
                channel_id: Self::NetworkFrame.id(),
                message_resend_time: Duration::ZERO,
                ..Default::default()
            }
                .into(),
            ReliableChannelConfig {
                channel_id: Self::ServerMessages.id(),
                message_resend_time: Duration::from_millis(200),
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