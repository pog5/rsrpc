//! Main RPC server coordinating all transports
//!
//! Handles Discord RPC commands and activity management.

use crate::bridge::Bridge;
use crate::config::Config;
use crate::process::ProcessScanner;
use crate::transports::ipc::IpcTransport;
use crate::transports::websocket::WebSocketTransport;
use crate::transports::{Transport, TransportCommand, TransportEvent};
use crate::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Main RPC server
pub struct RpcServer {
    config: Config,
    bridge: Arc<Bridge>,
    process_scanner: Arc<ProcessScanner>,
    /// Socket ID -> Client state
    clients: tokio::sync::RwLock<HashMap<SocketId, ClientState>>,
}

struct ClientState {
    client_id: String,
    last_pid: Option<Pid>,
}

impl RpcServer {
    pub fn new(config: Config) -> Self {
        let bridge = Arc::new(Bridge::new(config.clone()));
        let process_scanner = Arc::new(ProcessScanner::new(config.db_url.clone()));

        Self {
            config,
            bridge,
            process_scanner,
            clients: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Start the RPC server
    pub async fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("rsrpc v{} starting", env!("CARGO_PKG_VERSION"));

        // Initialize process scanner database
        if self.config.process_scanning {
            if let Err(e) = self.process_scanner.init().await {
                warn!("Failed to initialize process scanner: {}", e);
            }
        }

        // Start bridge servers
        self.bridge.clone().start().await?;

        // Event channel from transports
        let (event_tx, mut event_rx) = mpsc::channel::<TransportEvent>(256);

        // Start IPC transport
        let ipc = Arc::new(IpcTransport::new());
        let ipc_cmd_tx = ipc.clone().start(event_tx.clone()).await?;

        // Start WebSocket transport
        let ws = Arc::new(WebSocketTransport::new());
        let ws_cmd_tx = ws.clone().start(event_tx.clone()).await?;

        // Store command senders
        let cmd_senders: HashMap<&str, mpsc::Sender<TransportCommand>> =
            [("ipc", ipc_cmd_tx), ("ws", ws_cmd_tx)]
                .into_iter()
                .collect();

        // Start process scanner if enabled
        if self.config.process_scanning {
            let server = self.clone();
            let scan_interval = Duration::from_millis(self.config.scan_interval_ms);
            tokio::spawn(async move {
                server.run_process_scanner(scan_interval).await;
            });
        }

        // Handle events
        while let Some(event) = event_rx.recv().await {
            match event {
                TransportEvent::Connected {
                    socket_id,
                    client_id,
                } => {
                    info!("Client {} connected (client_id: {})", socket_id, client_id);

                    // Store client state
                    self.clients.write().await.insert(
                        socket_id,
                        ClientState {
                            client_id: client_id.clone(),
                            last_pid: None,
                        },
                    );

                    // Send READY event
                    let ready_response = self.create_ready_response();
                    self.send_to_client(&cmd_senders, socket_id, ready_response)
                        .await;
                }
                TransportEvent::Disconnected { socket_id } => {
                    info!("Client {} disconnected", socket_id);

                    // Clear activity
                    if let Some(client) = self.clients.write().await.remove(&socket_id) {
                        let msg = ActivityMessage {
                            activity: None,
                            pid: client.last_pid.unwrap_or(0),
                            socket_id: socket_id.to_string(),
                        };
                        self.bridge.send_activity(msg).await;
                    }
                }
                TransportEvent::Message { socket_id, command } => {
                    debug!("Received command from {}: {:?}", socket_id, command.cmd);

                    if let Some(response) = self.handle_command(socket_id, &command).await {
                        self.send_to_client(&cmd_senders, socket_id, response).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn send_to_client(
        &self,
        senders: &HashMap<&str, mpsc::Sender<TransportCommand>>,
        socket_id: SocketId,
        response: RpcResponse,
    ) {
        // Try all transports (they'll ignore if socket_id doesn't match)
        for tx in senders.values() {
            let _ = tx
                .send(TransportCommand::Send {
                    socket_id,
                    response: response.clone(),
                })
                .await;
        }
    }

    fn create_ready_response(&self) -> RpcResponse {
        let ready_data = ReadyData {
            v: 1,
            config: DiscordConfig::default(),
            user: User::default(),
        };

        RpcResponse {
            cmd: "DISPATCH".to_string(),
            data: Some(serde_json::to_value(ready_data).unwrap()),
            evt: Some("READY".to_string()),
            nonce: None,
        }
    }

    async fn handle_command(
        &self,
        socket_id: SocketId,
        command: &RpcCommand,
    ) -> Option<RpcResponse> {
        match command.cmd.as_str() {
            "SET_ACTIVITY" => self.handle_set_activity(socket_id, command).await,
            "CONNECTIONS_CALLBACK" => Some(RpcResponse {
                cmd: command.cmd.clone(),
                data: Some(serde_json::json!({ "code": 1000 })),
                evt: Some("ERROR".to_string()),
                nonce: command.nonce.clone(),
            }),
            "INVITE_BROWSER" | "GUILD_TEMPLATE_BROWSER" => {
                self.handle_browser_command(command).await
            }
            "DEEP_LINK" => Some(RpcResponse {
                cmd: command.cmd.clone(),
                data: None,
                evt: None,
                nonce: command.nonce.clone(),
            }),
            _ => {
                warn!("Unknown command: {}", command.cmd);
                None
            }
        }
    }

    async fn handle_set_activity(
        &self,
        socket_id: SocketId,
        command: &RpcCommand,
    ) -> Option<RpcResponse> {
        // Parse arguments
        let args: SetActivityArgs = match serde_json::from_value(command.args.clone()) {
            Ok(args) => args,
            Err(e) => {
                warn!("Failed to parse SET_ACTIVITY args: {}", e);
                return None;
            }
        };

        // Update client state
        {
            let mut clients = self.clients.write().await;
            if let Some(client) = clients.get_mut(&socket_id) {
                client.last_pid = Some(args.pid);
            }
        }

        // Get client ID
        let client_id = {
            let clients = self.clients.read().await;
            clients.get(&socket_id).map(|c| c.client_id.clone())
        }
        .unwrap_or_default();

        if let Some(mut activity) = args.activity {
            // Normalize timestamps (s -> ms)
            if let Some(ref mut timestamps) = activity.timestamps {
                let now_str = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis().to_string().len())
                    .unwrap_or(13);

                if let Some(start) = timestamps.start {
                    if now_str - start.to_string().len() > 2 {
                        timestamps.start = Some(start * 1000);
                    }
                }
                if let Some(end) = timestamps.end {
                    if now_str - end.to_string().len() > 2 {
                        timestamps.end = Some(end * 1000);
                    }
                }
            }

            // Handle buttons -> metadata
            if let Some(buttons) = &activity.buttons {
                activity.metadata.insert(
                    "button_urls".to_string(),
                    serde_json::to_value(buttons.iter().map(|b| &b.url).collect::<Vec<_>>())
                        .unwrap(),
                );
            }

            // Set application_id
            activity.application_id = Some(client_id.clone());

            // Set flags for instance
            if activity.instance == Some(true) {
                activity.flags = 1 << 0;
            }

            // Send to bridge
            let msg = ActivityMessage {
                activity: Some(activity.clone()),
                pid: args.pid,
                socket_id: socket_id.to_string(),
            };
            self.bridge.send_activity(msg).await;

            // Build response
            let response_data = serde_json::json!({
                "name": "",
                "application_id": client_id,
                "type": 0,
                "state": activity.state,
                "details": activity.details,
                "timestamps": activity.timestamps,
                "assets": activity.assets,
                "party": activity.party,
                "buttons": activity.buttons.as_ref().map(|b| b.iter().map(|x| &x.label).collect::<Vec<_>>()),
            });

            Some(RpcResponse {
                cmd: command.cmd.clone(),
                data: Some(response_data),
                evt: None,
                nonce: command.nonce.clone(),
            })
        } else {
            // Clear activity
            let msg = ActivityMessage {
                activity: None,
                pid: args.pid,
                socket_id: socket_id.to_string(),
            };
            self.bridge.send_activity(msg).await;

            Some(RpcResponse {
                cmd: command.cmd.clone(),
                data: None,
                evt: None,
                nonce: command.nonce.clone(),
            })
        }
    }

    async fn handle_browser_command(&self, command: &RpcCommand) -> Option<RpcResponse> {
        let code = command
            .args
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // TODO: Validate invite/guild template code
        // For now, just accept all codes
        Some(RpcResponse {
            cmd: command.cmd.clone(),
            data: Some(serde_json::json!({ "code": code })),
            evt: None,
            nonce: command.nonce.clone(),
        })
    }

    async fn run_process_scanner(&self, interval: Duration) {
        info!("Process scanner started (interval: {:?})", interval);

        loop {
            let detected = self.process_scanner.scan().await;

            for game in detected {
                // Create activity message for detected game
                let msg = ActivityMessage {
                    activity: Some(Activity {
                        application_id: Some(game.id.clone()),
                        name: Some(game.name.clone()),
                        timestamps: Some(Timestamps {
                            start: Some(game.start_time),
                            end: None,
                        }),
                        ..Default::default()
                    }),
                    pid: game.pid,
                    socket_id: game.id.clone(),
                };
                self.bridge.send_activity(msg).await;
            }

            tokio::time::sleep(interval).await;
        }
    }
}
