//! rsrpc - High-performance Discord RPC server
//!
//! A Rust replacement for arRPC with dual JSON/MessagePack protocol support.

use rsrpc::{Config, RpcServer};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn setup_logging(debug: bool) {
    let filter = if debug {
        "rsrpc=debug,info"
    } else {
        "rsrpc=info,warn"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn print_banner() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        r#"
╭─────────────────────────────────────╮
│  ┳━┓┏━┓┳━┓┳━┓┏━┓                    │
│  ┃┳┛┗━┓┃┳┛┃━┛┃                      │
│  ┻━┛━━┛┻━┛┻  ┗━┛  v{}            │
│                                     │
│  High-performance Discord RPC       │
╰─────────────────────────────────────╯
"#,
        version
    );
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM for systemd)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully...");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::parse_args();
    setup_logging(config.debug);
    print_banner();

    let server = Arc::new(RpcServer::new(config));

    tokio::select! {
        result = server.start() => {
            if let Err(e) = result {
                warn!("Server error: {}", e);
            }
        }
        _ = shutdown_signal() => {
            info!("Shutdown complete");
        }
    }

    Ok(())
}
