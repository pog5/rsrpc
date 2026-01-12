//! Configuration for rsrpc server

use std::env;

/// Server configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// JSON bridge port (arRPC-compatible)
    pub bridge_port: u16,
    /// MessagePack bridge port (optimized)
    pub msgpack_port: u16,
    /// WebSocket RPC port range start
    pub ws_port_start: u16,
    /// WebSocket RPC port range end
    pub ws_port_end: u16,
    /// Enable process scanning for game detection
    pub process_scanning: bool,
    /// Process scan interval in milliseconds
    pub scan_interval_ms: u64,
    /// Enable debug logging
    pub debug: bool,
    /// Custom detectable.json URL
    pub db_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bridge_port: 1337,
            msgpack_port: 1338,
            ws_port_start: 6463,
            ws_port_end: 6472,
            process_scanning: true,
            scan_interval_ms: 5000,
            debug: false,
            db_url:
                "https://raw.githubusercontent.com/OpenAsar/arrpc/main/src/process/detectable.json"
                    .to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(port) = env::var("RSRPC_BRIDGE_PORT") {
            if let Ok(p) = port.parse() {
                config.bridge_port = p;
            }
        }

        if let Ok(port) = env::var("RSRPC_MSGPACK_PORT") {
            if let Ok(p) = port.parse() {
                config.msgpack_port = p;
            }
        }

        if env::var("RSRPC_NO_PROCESS_SCANNING").is_ok() {
            config.process_scanning = false;
        }

        if env::var("RSRPC_DEBUG").is_ok() {
            config.debug = true;
        }

        if let Ok(url) = env::var("RSRPC_DB_URL") {
            config.db_url = url;
        }

        if let Ok(interval) = env::var("RSRPC_SCAN_INTERVAL") {
            if let Ok(i) = interval.parse() {
                config.scan_interval_ms = i;
            }
        }

        config
    }
}
