//! Transport layer for Discord RPC
//!
//! Supports:
//! - IPC (Windows named pipes / Unix sockets)
//! - WebSocket (ports 6463-6472)

pub mod ipc;
pub mod websocket;

use crate::types::{RpcCommand, RpcResponse, SocketId};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Events from transport layer to server
#[derive(Debug)]
pub enum TransportEvent {
    /// New client connected
    Connected {
        socket_id: SocketId,
        client_id: String,
    },
    /// Client disconnected
    Disconnected { socket_id: SocketId },
    /// Message received from client
    Message {
        socket_id: SocketId,
        command: RpcCommand,
    },
}

/// Commands from server to transport layer
#[derive(Debug, Clone)]
pub enum TransportCommand {
    /// Send response to specific client
    Send {
        socket_id: SocketId,
        response: RpcResponse,
    },
    /// Close connection to client
    Close { socket_id: SocketId },
}

/// Transport trait for different connection types
#[allow(async_fn_in_trait)]
pub trait Transport: Send + Sync {
    /// Start the transport, returning event sender
    async fn start(
        self: Arc<Self>,
        event_tx: mpsc::Sender<TransportEvent>,
    ) -> Result<mpsc::Sender<TransportCommand>, std::io::Error>;

    /// Get transport name for logging
    fn name(&self) -> &'static str;
}
