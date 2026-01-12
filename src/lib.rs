//! rsrpc - High-performance Discord RPC server replacement
//!
//! Dual protocol support:
//! - JSON (arRPC-compatible) on port 1337
//! - MessagePack (optimized) on port 1338

pub mod bridge;
pub mod config;
pub mod process;
pub mod protocol;
pub mod server;
pub mod transports;
pub mod types;

pub use config::Config;
pub use server::RpcServer;
pub use types::*;
