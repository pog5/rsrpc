//! Configuration for rsrpc server

use clap::Parser;

/// Server configuration
#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// JSON bridge port (arRPC-compatible)
    #[arg(long, env = "RSRPC_BRIDGE_PORT", default_value_t = 1337)]
    pub bridge_port: u16,

    /// MessagePack bridge port (optimized)
    #[arg(long, env = "RSRPC_MSGPACK_PORT", default_value_t = 1338)]
    pub msgpack_port: u16,

    /// WebSocket RPC port range start
    #[arg(long, env = "RSRPC_WS_PORT_START", default_value_t = 6463)]
    pub ws_port_start: u16,

    /// WebSocket RPC port range end
    #[arg(long, env = "RSRPC_WS_PORT_END", default_value_t = 6472)]
    pub ws_port_end: u16,

    /// Disable process scanning for game detection
    #[arg(long, env = "RSRPC_NO_PROCESS_SCANNING")]
    pub no_process_scanning: bool,

    /// Process scan interval in milliseconds
    #[arg(long, env = "RSRPC_SCAN_INTERVAL", default_value_t = 5000)]
    pub scan_interval_ms: u64,

    /// Enable debug logging
    #[arg(long, short, env = "RSRPC_DEBUG")]
    pub debug: bool,

    /// Custom detectable.json URL
    #[arg(
        long,
        env = "RSRPC_DB_URL",
        default_value = "https://discord.com/api/v9/applications/detectable"
    )]
    pub db_url: String,

    /// Enable dynamic update of detectable.json at startup
    #[arg(long, env = "RSRPC_ENABLE_DB_UPDATE")]
    pub enable_db_update: bool,
}

impl Config {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bridge_port: 1337,
            msgpack_port: 1338,
            ws_port_start: 6463,
            ws_port_end: 6472,
            no_process_scanning: false, // Default is scanning enabled (so "no" is false)
            scan_interval_ms: 5000,
            debug: false,
            db_url: "https://discord.com/api/v9/applications/detectable".to_string(),
            enable_db_update: false,
        }
    }
}
