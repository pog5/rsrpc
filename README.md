# rsrpc~ 

![License](https://img.shields.io/badge/license-GPLv3-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-windows%20%7C%20linux-lightgrey.svg)
![Status](https://img.shields.io/badge/status-stable-green.svg)

**rsrpc** is a high-performance, drop-in replacement for [arRPC](https://github.com/OpenAsar/arrpc) written in Rust. It provides a Discord Rich Presence server with significantly lower resource usage, faster process detection, and dual protocol support.

## 🚀 Features

- **Drop-in arRPC Replacement**: 100% compatible with arRPC's JSON interface (WebSocket port 1337).
- **MessagePack Support**: Optional secondary server on port 1338 using MessagePack for ~25% smaller payloads.
- **Fast Process Detection**: Uses `sysinfo` native bindings (faster than `wmic` subprocess calls, doesn't spawn subprocesses).
- **Dual Transports**: Supports both IPC (named pipes/unix sockets) and WebSocket (ports 6463-6472).

## 🛠️ Installation & Building

### Prerequisites
- Windows or Linux
- Rust and Cargo (for building from source)

### Installation
**Binaries**: 
Grab the latest nightly binary from the [Releases](https://github.com/yourusername/rsrpc/releases) page. We provide builds for Windows (x64, arm64) and Linux (x64, arm64).

**From Source**:
```bash
git clone https://github.com/yourusername/rsrpc.git
cd rsrpc
cargo build --release
```

## 📖 Usage

```bash
./rsrpc [OPTIONS]
```

### Configuration
rsrpc can be configured via Command Line Arguments (flags) or Environment Variables. Flags take precedence.

| Flag | Env Variable | Default | Description |
|------|--------------|---------|-------------|
| `--bridge-port <PORT>` | `RSRPC_BRIDGE_PORT` | `1337` | Port for JSON bridge |
| `--msgpack-port <PORT>` | `RSRPC_MSGPACK_PORT` | `1338` | Port for MessagePack bridge |
| `--enable-db-update` | `RSRPC_ENABLE_DB_UPDATE` | `false` | Enable startup DB fetch |
| `--no-process-scanning` | `RSRPC_NO_PROCESS_SCANNING` | `false` | Disable game detection |
| `--debug` | `RSRPC_DEBUG` | `false` | Enable verbose logging |

### Database Updates
By default, rsrpc uses a **bundled** database of games for instant startup and zero network dependency.
To enable runtime updates (fetching the latest list from Discord):
```bash
./rsrpc --enable-db-update
```

## 🔌 Integration

### Vencord / Vesktop
rsrpc works out of the box with Vencord's **WebRichPresence** plugin due to its arRPC compatibility. Just enable the plugin and ensure rsrpc is running.

### Custom Clients (JavaScript)

You can connect using standard WebSocket:

```javascript
// Connect to standard JSON bridge
const ws = new WebSocket('ws://127.0.0.1:1337');
ws.onmessage = (msg) => console.log(JSON.parse(msg.data));
```

Or use the optimized MessagePack interface (requires `@msgpack/msgpack`):

```javascript
import { decode } from '@msgpack/msgpack';
const ws = new WebSocket('ws://127.0.0.1:1338?format=msgpack');
ws.binaryType = 'arraybuffer';
ws.onmessage = (msg) => console.log(decode(new Uint8Array(msg.data)));
```

## 🏗️ Architecture

- **Server**: Orchestrates transports and process scanning.
- **Transports**:
  - **IPC**: Handles local Discord client communication (`\\.\pipe\discord-ipc-0` or `/tmp/discord-ipc-0`).
  - **WebSocket**: Mimics local Discord WebSocket server (ports 6463-6472).
- **Process Scanner**: Periodically scans running processes against a local cache of `detectable.json`.
- **Bridge**: Broadcasts activity updates to connected web clients (e.g., Vencord).

## 📊 Comparison vs arRPC

| Feature | arRPC (Node.js) | rsrpc (Rust) |
|---------|-----------------|--------------|
| **Memory Usage** | 10GB. | 10MB |
| **Executable Size** | ~125MB (when bundled with Bun) | ~3 MB (Standalone, Windows) |
| **Process Scan** | `powershell` + `wmic` | `sysinfo` |
| **Protocols** | JSON | JSON + MessagePack |

## 📜 License

GPLv3 License. See [LICENSE](LICENSE) for details.
