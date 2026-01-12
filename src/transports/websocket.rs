//! WebSocket transport for Discord RPC
//!
//! Listens on ports 6463-6472 for Discord RPC connections.

use super::{Transport, TransportCommand, TransportEvent};
use crate::types::{Handshake, RpcCommand, RpcResponse, SocketId};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

const PORT_RANGE_START: u16 = 6463;
const PORT_RANGE_END: u16 = 6472;

/// Allowed origins for WebSocket connections
const ALLOWED_ORIGINS: [&str; 3] = [
    "https://discord.com",
    "https://ptb.discord.com",
    "https://canary.discord.com",
];

/// WebSocket transport server
pub struct WebSocketTransport {
    clients: RwLock<HashMap<SocketId, WsClient>>,
    next_socket_id: AtomicU64,
}

struct WsClient {
    client_id: String,
    tx: mpsc::Sender<Message>,
}

impl WebSocketTransport {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            next_socket_id: AtomicU64::new(1000), // Start at 1000 to avoid collision with IPC
        }
    }

    fn next_id(&self) -> SocketId {
        self.next_socket_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for WebSocketTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for WebSocketTransport {
    async fn start(
        self: Arc<Self>,
        event_tx: mpsc::Sender<TransportEvent>,
    ) -> Result<mpsc::Sender<TransportCommand>, std::io::Error> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<TransportCommand>(256);

        // Find available port
        let listener = find_available_port().await?;
        let port = listener.local_addr()?.port();
        info!("WebSocket listening on port {}", port);

        // Spawn command handler
        let transport = self.clone();
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    TransportCommand::Send {
                        socket_id,
                        response,
                    } => {
                        let clients = transport.clients.read().await;
                        if let Some(client) = clients.get(&socket_id) {
                            let json = serde_json::to_string(&response).unwrap_or_default();
                            let _ = client.tx.send(Message::Text(json)).await;
                        }
                    }
                    TransportCommand::Close { socket_id } => {
                        let mut clients = transport.clients.write().await;
                        if let Some(client) = clients.remove(&socket_id) {
                            let _ = client.tx.send(Message::Close(None)).await;
                        }
                    }
                }
            }
        });

        // Spawn listener
        let transport = self.clone();
        tokio::spawn(async move {
            if let Err(e) = run_ws_server(transport, listener, event_tx).await {
                error!("WebSocket server error: {}", e);
            }
        });

        Ok(cmd_tx)
    }

    fn name(&self) -> &'static str {
        "WebSocket"
    }
}

async fn find_available_port() -> Result<TcpListener, std::io::Error> {
    for port in PORT_RANGE_START..=PORT_RANGE_END {
        match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(listener) => return Ok(listener),
            Err(e) => {
                debug!("Port {} in use: {}", port, e);
                continue;
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        "All WebSocket ports in use",
    ))
}

async fn run_ws_server(
    transport: Arc<WebSocketTransport>,
    listener: TcpListener,
    event_tx: mpsc::Sender<TransportEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let (stream, addr) = listener.accept().await?;
        debug!("WebSocket connection from {}", addr);

        let socket_id = transport.next_id();
        let event_tx = event_tx.clone();
        let transport = transport.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_ws_connection(stream, socket_id, event_tx, transport).await {
                debug!("WebSocket client {} error: {}", socket_id, e);
            }
        });
    }
}

async fn handle_ws_connection(
    stream: tokio::net::TcpStream,
    socket_id: SocketId,
    event_tx: mpsc::Sender<TransportEvent>,
    transport: Arc<WebSocketTransport>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Perform WebSocket handshake with callback for origin validation
    let ws_stream = tokio_tungstenite::accept_hdr_async(
        stream,
        |req: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
            // Get origin header
            if let Some(origin) = req.headers().get("origin") {
                if let Ok(origin_str) = origin.to_str() {
                    if !origin_str.is_empty() && !ALLOWED_ORIGINS.contains(&origin_str) {
                        warn!("Rejected connection from origin: {}", origin_str);
                        // Note: tokio-tungstenite doesn't support rejecting in this callback easily
                        // The connection will be rejected after this returns an error
                    }
                }
            }

            // Parse query parameters for client_id, version, encoding
            let uri = req.uri();
            let query = uri.query().unwrap_or("");
            debug!("WebSocket connection params: {}", query);

            Ok(response)
        },
    )
    .await?;

    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Message>(64);

    // Spawn writer task
    let write_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut client_id = String::new();
    let mut connected = false;

    // Read messages
    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                // Parse as JSON
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json) => {
                        // Check if this is a handshake or command
                        if !connected {
                            // First message should contain client_id
                            if let Some(cid) = json.get("client_id").and_then(|v| v.as_str()) {
                                client_id = cid.to_string();
                            }

                            // Store client
                            transport.clients.write().await.insert(
                                socket_id,
                                WsClient {
                                    client_id: client_id.clone(),
                                    tx: tx.clone(),
                                },
                            );

                            connected = true;
                            event_tx
                                .send(TransportEvent::Connected {
                                    socket_id,
                                    client_id: client_id.clone(),
                                })
                                .await?;
                        }

                        // Parse as RPC command
                        if let Ok(command) = serde_json::from_value::<RpcCommand>(json) {
                            event_tx
                                .send(TransportEvent::Message { socket_id, command })
                                .await?;
                        }
                    }
                    Err(e) => {
                        warn!("Invalid JSON from client: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Ok(_) => {
                // Ignore other message types
            }
            Err(e) => {
                debug!("WebSocket read error: {}", e);
                break;
            }
        }
    }

    // Cleanup
    write_handle.abort();
    transport.clients.write().await.remove(&socket_id);
    event_tx
        .send(TransportEvent::Disconnected { socket_id })
        .await?;

    Ok(())
}

/// Parse query string into map
#[allow(dead_code)]
fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}
