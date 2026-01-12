//! Bridge WebSocket server for web clients
//!
//! Dual protocol support:
//! - Port 1337: JSON (arRPC-compatible)
//! - Port 1338: MessagePack (optimized)

use crate::config::Config;
use crate::protocol::{get_codec, Protocol};
use crate::types::ActivityMessage;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info};

/// Bridge server for web clients
pub struct Bridge {
    /// Last activity per socket for late joiners
    last_activities: RwLock<HashMap<String, ActivityMessage>>,
    /// Broadcast channel for activity updates
    activity_tx: broadcast::Sender<(ActivityMessage, Protocol)>,
    /// Configuration
    config: Config,
}

impl Bridge {
    pub fn new(config: Config) -> Self {
        let (activity_tx, _) = broadcast::channel(256);
        Self {
            last_activities: RwLock::new(HashMap::new()),
            activity_tx,
            config,
        }
    }

    /// Send activity update to all connected bridge clients
    pub async fn send_activity(&self, msg: ActivityMessage) {
        // Store for late joiners
        self.last_activities
            .write()
            .await
            .insert(msg.socket_id.clone(), msg.clone());

        // Broadcast to all
        let _ = self.activity_tx.send((msg.clone(), Protocol::Json));
        let _ = self.activity_tx.send((msg, Protocol::MsgPack));
    }

    /// Clear activity for socket
    pub async fn clear_activity(&self, socket_id: &str) {
        self.last_activities.write().await.remove(socket_id);
    }

    /// Start bridge servers
    pub async fn start(self: Arc<Self>) -> Result<(), std::io::Error> {
        // Start JSON bridge (arRPC-compatible)
        let json_bridge = self.clone();
        let json_port = self.config.bridge_port;
        tokio::spawn(async move {
            if let Err(e) = json_bridge.run_server(json_port, Protocol::Json).await {
                error!("JSON bridge error: {}", e);
            }
        });

        // Start MessagePack bridge (optimized)
        let msgpack_bridge = self.clone();
        let msgpack_port = self.config.msgpack_port;
        tokio::spawn(async move {
            if let Err(e) = msgpack_bridge
                .run_server(msgpack_port, Protocol::MsgPack)
                .await
            {
                error!("MessagePack bridge error: {}", e);
            }
        });

        Ok(())
    }

    async fn run_server(
        self: Arc<Self>,
        port: u16,
        default_protocol: Protocol,
    ) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        let protocol_name = match default_protocol {
            Protocol::Json => "JSON",
            Protocol::MsgPack => "MessagePack",
        };
        info!("Bridge ({}) listening on port {}", protocol_name, port);

        loop {
            let (stream, addr) = listener.accept().await?;
            debug!("Bridge connection from {}", addr);

            let bridge = self.clone();
            let protocol = default_protocol;

            tokio::spawn(async move {
                if let Err(e) = bridge.handle_connection(stream, protocol).await {
                    debug!("Bridge client error: {}", e);
                }
            });
        }
    }

    async fn handle_connection(
        self: Arc<Self>,
        stream: tokio::net::TcpStream,
        default_protocol: Protocol,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Accept WebSocket with query string detection
        let mut detected_protocol = default_protocol;

        let ws_stream = tokio_tungstenite::accept_hdr_async(
            stream,
            |req: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
                // Check query params for protocol override
                let uri = req.uri();
                if let Some(query) = uri.query() {
                    if let Some(proto) = Protocol::from_query(query) {
                        detected_protocol = proto;
                    }
                }
                Ok(response)
            },
        )
        .await?;

        let protocol = detected_protocol;
        let codec = get_codec(protocol);

        let (mut write, mut read) = ws_stream.split();
        let mut activity_rx = self.activity_tx.subscribe();

        info!("Bridge client connected ({:?})", protocol);

        // Send cached activities to late joiner
        {
            let activities = self.last_activities.read().await;
            for msg in activities.values() {
                if msg.activity.is_some() {
                    let data = codec.encode_activity(msg)?;
                    let ws_msg = match protocol {
                        Protocol::Json => Message::Text(String::from_utf8_lossy(&data).to_string()),
                        Protocol::MsgPack => Message::Binary(data),
                    };
                    write.send(ws_msg).await?;
                }
            }
        }

        // Main loop: forward activities to client
        loop {
            tokio::select! {
                // Receive activity broadcast
                result = activity_rx.recv() => {
                    match result {
                        Ok((msg, msg_protocol)) => {
                            // Only send if protocol matches
                            if msg_protocol == protocol {
                                let data = codec.encode_activity(&msg)?;
                                let ws_msg = match protocol {
                                    Protocol::Json => Message::Text(String::from_utf8_lossy(&data).to_string()),
                                    Protocol::MsgPack => Message::Binary(data),
                                };
                                if write.send(ws_msg).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // Missed some messages, continue
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                // Check for client disconnect
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Close(_))) | None => {
                            break;
                        }
                        Some(Err(_)) => {
                            break;
                        }
                        _ => {
                            // Ignore other messages from client
                        }
                    }
                }
            }
        }

        info!("Bridge client disconnected");
        Ok(())
    }
}
