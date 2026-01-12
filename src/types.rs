//! Core types for Discord RPC protocol
//!
//! Optimized for minimal allocations and efficient serialization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Process ID type
pub type Pid = u32;

/// Socket ID for client identification
pub type SocketId = u64;

/// Discord application/client ID
pub type ClientId = String;

/// IPC message types (Discord RPC protocol)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IpcOpcode {
    Handshake = 0,
    Frame = 1,
    Close = 2,
    Ping = 3,
    Pong = 4,
}

impl TryFrom<u32> for IpcOpcode {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Handshake),
            1 => Ok(Self::Frame),
            2 => Ok(Self::Close),
            3 => Ok(Self::Ping),
            4 => Ok(Self::Pong),
            _ => Err(()),
        }
    }
}

/// IPC close codes
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum CloseCode {
    Normal = 1000,
    Unsupported = 1003,
    Abnormal = 1006,
}

/// IPC error codes
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum ErrorCode {
    InvalidClientId = 4000,
    InvalidOrigin = 4001,
    RateLimited = 4002,
    TokenRevoked = 4003,
    InvalidVersion = 4004,
    InvalidEncoding = 4005,
}

/// RPC command from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCommand {
    pub cmd: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub nonce: Option<String>,
}

/// RPC response to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

/// Handshake request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub v: u32,
    pub client_id: String,
}

/// Activity timestamps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Timestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
}

/// Activity assets (images)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Assets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

/// Activity party info
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Party {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<[u32; 2]>,
}

/// Activity button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    pub label: String,
    pub url: String,
}

/// Discord Rich Presence activity
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Activity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Assets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<Party>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<Button>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<bool>,
    #[serde(rename = "type", default)]
    pub activity_type: u8,
    #[serde(default)]
    pub flags: u32,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// SET_ACTIVITY command arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetActivityArgs {
    pub pid: Pid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

/// Activity message sent to bridge clients (arRPC format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
    pub pid: Pid,
    #[serde(rename = "socketId")]
    pub socket_id: String,
}

/// READY dispatch data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyData {
    pub v: u32,
    pub config: DiscordConfig,
    pub user: User,
}

/// Discord config in READY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub cdn_host: String,
    pub api_endpoint: String,
    pub environment: String,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            cdn_host: "cdn.discordapp.com".to_string(),
            api_endpoint: "//discord.com/api".to_string(),
            environment: "production".to_string(),
        }
    }
}

/// Mock user for READY (arRPC compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub global_name: String,
    pub avatar: String,
    pub avatar_decoration_data: Option<()>,
    pub bot: bool,
    pub flags: u32,
    pub premium_type: u32,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: "1045800378228281345".to_string(),
            username: "rsrpc".to_string(),
            discriminator: "0".to_string(),
            global_name: "rsRPC".to_string(),
            avatar: "cfefa4d9839fb4bdf030f91c2a13e95c".to_string(),
            avatar_decoration_data: None,
            bot: false,
            flags: 0,
            premium_type: 0,
        }
    }
}

/// Process information for game detection
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub path: PathBuf,
    pub args: Option<String>,
}

/// Cache for detected processes and activities
#[derive(Debug, Default)]
pub struct ProcessCache {
    /// PID -> executable path
    pub processes: HashMap<Pid, PathBuf>,
    /// App ID -> start timestamp
    pub timestamps: HashMap<String, i64>,
    /// App ID -> game name
    pub names: HashMap<String, String>,
    /// App ID -> PID
    pub pids: HashMap<String, Pid>,
}

/// Detectable game from arRPC database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectableGame {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub executables: Vec<Executable>,
}

/// Executable entry in detectable database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Executable {
    pub name: String,
    #[serde(default)]
    pub is_launcher: bool,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}
