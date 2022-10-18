use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;
use bevy::prelude::{Component, Entity};
use renet::{ChannelConfig, NETCODE_KEY_BYTES, ReliableChannelConfig, RenetConnectionConfig, UnreliableChannelConfig};

pub const PORT: i32 = 5000;
pub const AMOUNT_PLAYERS: usize = 4;

pub const PRIVATE_KEY: &[u8; NETCODE_KEY_BYTES] = b"an example very very secret key.";
pub const PROTOCOL_ID: u64 = 6969;

pub const TICKRATE: u64 = 250;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct Tick(pub Option<i128>);

impl Tick {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn get(&self) -> i128 {
        self.0.unwrap()
    }

    pub fn set(&mut self, tick: i128) {
        self.0 = Some(tick);
    }

    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    pub fn increment(&mut self) {
        if self.is_set() {
            self.0 = Some(self.get() + 1);
        } else {
            self.0 = Some(0);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u64);

impl Display for PlayerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Clients last received ticks
#[derive(Debug, Default)]
pub struct ClientTicks(pub HashMap<PlayerId, Tick>);

#[derive(Debug, Component)]
pub struct Player {
    pub id: PlayerId,
}

#[derive(Debug, Default)]
pub struct ServerLobby(pub HashMap<PlayerId, Entity>);

#[derive(Debug, Default)]
pub struct ClientLobby {
    pub players: HashMap<u64, PlayerInfo>,
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub client_entity: Entity,
    pub server_entity: Entity,
}

pub enum ClientType {
    Client,
    Server,
}

impl Debug for ClientType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientType::Client => write!(f, "Client"),
            ClientType::Server => write!(f, "Server"),
        }
    }
}

impl Display for ClientType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientType::Client => write!(f, "Client"),
            ClientType::Server => write!(f, "Server"),
        }
    }
}

pub enum ClientChannel {
    Input,
    Command,
}

pub enum ServerChannel {
    ServerMessages,
    NetworkFrame,
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
            UnreliableChannelConfig {
                channel_id: Self::NetworkFrame.id(),
                // message_resend_time: Duration::ZERO,
                message_receive_queue_size: 2048,
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