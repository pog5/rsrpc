//! IPC transport for Discord RPC
//!
//! Windows: Named pipes at `\\?\pipe\discord-ipc-{0-9}`
//! Unix: Sockets at `$XDG_RUNTIME_DIR/discord-ipc-{0-9}` or `/tmp/discord-ipc-{0-9}`

use super::{Transport, TransportCommand, TransportEvent};
use crate::types::{CloseCode, Handshake, IpcOpcode, RpcCommand, RpcResponse, SocketId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

#[cfg(unix)]
use tokio::net::UnixListener;

/// IPC transport server
pub struct IpcTransport {
    /// Connected clients
    clients: RwLock<HashMap<SocketId, IpcClient>>,
    /// Next socket ID counter
    next_socket_id: AtomicU64,
}

struct IpcClient {
    #[allow(dead_code)] // used in recieving end
    client_id: String,
    tx: mpsc::Sender<Vec<u8>>,
}

impl IpcTransport {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            next_socket_id: AtomicU64::new(0),
        }
    }

    fn next_id(&self) -> SocketId {
        self.next_socket_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for IpcTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode IPC message with header
fn encode_message(opcode: IpcOpcode, data: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(8 + data.len());
    buffer.extend_from_slice(&(opcode as u32).to_le_bytes());
    buffer.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buffer.extend_from_slice(data);
    buffer
}

/// Encode RPC response
fn encode_response(response: &RpcResponse) -> Vec<u8> {
    let json = serde_json::to_vec(response).unwrap_or_default();
    encode_message(IpcOpcode::Frame, &json)
}

#[cfg(windows)]
const PIPE_PREFIX: &str = r"\\.\pipe\discord-ipc-";

#[cfg(unix)]
fn get_socket_path(index: u32) -> std::path::PathBuf {
    let base = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .or_else(|_| std::env::var("TMP"))
        .or_else(|_| std::env::var("TEMP"))
        .unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(base).join(format!("discord-ipc-{}", index))
}

impl Transport for IpcTransport {
    async fn start(
        self: Arc<Self>,
        event_tx: mpsc::Sender<TransportEvent>,
    ) -> Result<mpsc::Sender<TransportCommand>, std::io::Error> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<TransportCommand>(256);

        // Find available IPC slot (0-9)
        let slot = find_available_slot().await?;
        info!("IPC listening on slot {}", slot);

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
                            let data = encode_response(&response);
                            let _ = client.tx.send(data).await;
                        }
                    }
                    TransportCommand::Close { socket_id } => {
                        transport.clients.write().await.remove(&socket_id);
                    }
                }
            }
        });

        // Spawn listener
        let transport = self.clone();
        tokio::spawn(async move {
            if let Err(e) = run_ipc_server(transport, event_tx, slot).await {
                error!("IPC server error: {}", e);
            }
        });

        Ok(cmd_tx)
    }

    fn name(&self) -> &'static str {
        "IPC"
    }
}

#[cfg(windows)]
async fn find_available_slot() -> Result<u32, std::io::Error> {
    for i in 0..10 {
        let pipe_name = format!("{}{}", PIPE_PREFIX, i);
        // Try to create the pipe - if it succeeds, the slot is available
        match ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)
        {
            Ok(_) => return Ok(i),
            Err(_) => continue,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        "All IPC slots in use",
    ))
}

#[cfg(unix)]
async fn find_available_slot() -> Result<u32, std::io::Error> {
    for i in 0..10 {
        let path = get_socket_path(i);
        // Remove existing socket if present
        let _ = std::fs::remove_file(&path);
        match UnixListener::bind(&path) {
            Ok(_) => return Ok(i),
            Err(_) => continue,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        "All IPC slots in use",
    ))
}

#[cfg(windows)]
async fn run_ipc_server(
    transport: Arc<IpcTransport>,
    event_tx: mpsc::Sender<TransportEvent>,
    slot: u32,
) -> Result<(), std::io::Error> {
    let pipe_name = format!("{}{}", PIPE_PREFIX, slot);
    info!("IPC server starting at {}", pipe_name);

    loop {
        // Create new pipe instance
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(&pipe_name)?;

        // Wait for connection
        server.connect().await?;
        info!("IPC client connected");

        let socket_id = transport.next_id();
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
        let event_tx = event_tx.clone();
        let transport = transport.clone();

        tokio::spawn(async move {
            if let Err(e) =
                handle_ipc_client(server, socket_id, tx, &mut rx, event_tx, transport).await
            {
                debug!("IPC client {} disconnected: {}", socket_id, e);
            }
        });
    }
}

#[cfg(unix)]
async fn run_ipc_server(
    transport: Arc<IpcTransport>,
    event_tx: mpsc::Sender<TransportEvent>,
    slot: u32,
) -> Result<(), std::io::Error> {
    let path = get_socket_path(slot);
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    info!("IPC server listening at {:?}", path);

    loop {
        let (stream, _) = listener.accept().await?;
        info!("IPC client connected");

        let socket_id = transport.next_id();
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
        let event_tx = event_tx.clone();
        let transport = transport.clone();

        tokio::spawn(async move {
            if let Err(e) =
                handle_ipc_client_unix(stream, socket_id, tx, &mut rx, event_tx, transport).await
            {
                debug!("IPC client {} disconnected: {}", socket_id, e);
            }
        });
    }
}

#[cfg(windows)]
async fn handle_ipc_client(
    mut pipe: NamedPipeServer,
    socket_id: SocketId,
    tx: mpsc::Sender<Vec<u8>>,
    rx: &mut mpsc::Receiver<Vec<u8>>,
    event_tx: mpsc::Sender<TransportEvent>,
    transport: Arc<IpcTransport>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handshook = false;
    let mut ipc_client_id;

    loop {
        tokio::select! {
            // Read from pipe
            result = read_ipc_message(&mut pipe) => {
                match result {
                    Ok((opcode, data)) => {
                        match opcode {
                            IpcOpcode::Handshake => {
                                if handshook {
                                    warn!("Already handshook");
                                    continue;
                                }
                                let handshake: Handshake = serde_json::from_slice(&data)?;
                                if handshake.v != 1 {
                                    send_close(&mut pipe, CloseCode::Unsupported, "Invalid version").await?;
                                    break;
                                }
                                ipc_client_id = handshake.client_id.clone();
                                handshook = true;

                                // Store client
                                transport.clients.write().await.insert(socket_id, IpcClient {
                                    client_id: ipc_client_id.clone(),
                                    tx: tx.clone(),
                                });

                                // Notify server
                                event_tx.send(TransportEvent::Connected {
                                    socket_id,
                                    client_id: ipc_client_id.clone(),
                                }).await?;
                            }
                            IpcOpcode::Frame => {
                                if !handshook {
                                    warn!("Need to handshake first");
                                    continue;
                                }
                                let command: RpcCommand = serde_json::from_slice(&data)?;
                                event_tx.send(TransportEvent::Message {
                                    socket_id,
                                    command,
                                }).await?;
                            }
                            IpcOpcode::Ping => {
                                // Respond with pong
                                let pong = encode_message(IpcOpcode::Pong, &data);
                                pipe.write_all(&pong).await?;
                            }
                            IpcOpcode::Close => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        debug!("IPC read error: {}", e);
                        break;
                    }
                }
            }
            // Write to pipe
            Some(data) = rx.recv() => {
                pipe.write_all(&data).await?;
            }
        }
    }

    // Cleanup
    transport.clients.write().await.remove(&socket_id);
    event_tx
        .send(TransportEvent::Disconnected { socket_id })
        .await?;
    Ok(())
}

#[cfg(unix)]
async fn handle_ipc_client_unix(
    mut stream: tokio::net::UnixStream,
    socket_id: SocketId,
    tx: mpsc::Sender<Vec<u8>>,
    rx: &mut mpsc::Receiver<Vec<u8>>,
    event_tx: mpsc::Sender<TransportEvent>,
    transport: Arc<IpcTransport>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handshook = false;
    let mut client_id = String::new();

    loop {
        tokio::select! {
            result = read_ipc_message_unix(&mut stream) => {
                match result {
                    Ok((opcode, data)) => {
                        match opcode {
                            IpcOpcode::Handshake => {
                                if handshook {
                                    warn!("Already handshook");
                                    continue;
                                }
                                let handshake: Handshake = serde_json::from_slice(&data)?;
                                if handshake.v != 1 {
                                    send_close_unix(&mut stream, CloseCode::Unsupported, "Invalid version").await?;
                                    break;
                                }
                                client_id = handshake.client_id.clone();
                                handshook = true;

                                transport.clients.write().await.insert(socket_id, IpcClient {
                                    client_id: client_id.clone(),
                                    tx: tx.clone(),
                                });

                                event_tx.send(TransportEvent::Connected {
                                    socket_id,
                                    client_id: client_id.clone(),
                                }).await?;
                            }
                            IpcOpcode::Frame => {
                                if !handshook {
                                    warn!("Need to handshake first");
                                    continue;
                                }
                                let command: RpcCommand = serde_json::from_slice(&data)?;
                                event_tx.send(TransportEvent::Message {
                                    socket_id,
                                    command,
                                }).await?;
                            }
                            IpcOpcode::Ping => {
                                let pong = encode_message(IpcOpcode::Pong, &data);
                                stream.write_all(&pong).await?;
                            }
                            IpcOpcode::Close => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        debug!("IPC read error: {}", e);
                        break;
                    }
                }
            }
            Some(data) = rx.recv() => {
                stream.write_all(&data).await?;
            }
        }
    }

    transport.clients.write().await.remove(&socket_id);
    event_tx
        .send(TransportEvent::Disconnected { socket_id })
        .await?;
    Ok(())
}

#[cfg(windows)]
async fn read_ipc_message(
    pipe: &mut NamedPipeServer,
) -> Result<(IpcOpcode, Vec<u8>), std::io::Error> {
    let mut header = [0u8; 8];
    pipe.read_exact(&mut header).await?;

    let opcode = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    let opcode = IpcOpcode::try_from(opcode)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid opcode"))?;

    let mut data = vec![0u8; size];
    pipe.read_exact(&mut data).await?;

    Ok((opcode, data))
}

#[cfg(unix)]
async fn read_ipc_message_unix(
    stream: &mut tokio::net::UnixStream,
) -> Result<(IpcOpcode, Vec<u8>), std::io::Error> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await?;

    let opcode = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    let opcode = IpcOpcode::try_from(opcode)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid opcode"))?;

    let mut data = vec![0u8; size];
    stream.read_exact(&mut data).await?;

    Ok((opcode, data))
}

#[cfg(windows)]
async fn send_close(
    pipe: &mut NamedPipeServer,
    code: CloseCode,
    message: &str,
) -> Result<(), std::io::Error> {
    let payload = serde_json::json!({
        "code": code as u32,
        "message": message
    });
    let json = serde_json::to_vec(&payload).unwrap();
    let msg = encode_message(IpcOpcode::Close, &json);
    pipe.write_all(&msg).await
}

#[cfg(unix)]
async fn send_close_unix(
    stream: &mut tokio::net::UnixStream,
    code: CloseCode,
    message: &str,
) -> Result<(), std::io::Error> {
    let payload = serde_json::json!({
        "code": code as u32,
        "message": message
    });
    let json = serde_json::to_vec(&payload).unwrap();
    let msg = encode_message(IpcOpcode::Close, &json);
    stream.write_all(&msg).await
}
