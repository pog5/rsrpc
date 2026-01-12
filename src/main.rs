//! rsrpc - High-performance Discord RPC server
//!
//! A Rust replacement for arRPC with dual JSON/MessagePack protocol support.

use rsrpc::{Config, RpcServer};
use std::sync::Arc;
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
│  ┻━┛━━┛┻━┛┻  ┗━┛  v{}             │
│                                     │
│  High-performance Discord RPC       │
│  Dual Protocol: JSON + MessagePack  │
╰─────────────────────────────────────╯
"#,
        version
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::from_env();
    setup_logging(config.debug);
    print_banner();

    let server = Arc::new(RpcServer::new(config));
    server.start().await?;

    Ok(())
}
