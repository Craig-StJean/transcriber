mod api;
mod audio;
mod db;
mod dbus;

use anyhow::Result;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("voice-transcriber-daemon starting");

    let cfg = common::config::load()?;
    tracing::info!("config loaded from {}", common::config::config_path().display());

    if cfg.active_key().is_empty() {
        tracing::warn!(
            "api_key is empty — transcription will fail until you set it in {}",
            common::config::config_path().display()
        );
    }

    let db = Arc::new(db::Database::open()?);

    let iface = dbus::TranscriberInterface {
        state:       Arc::new(Mutex::new(dbus::DaemonState::Idle)),
        config:      Arc::new(Mutex::new(cfg)),
        audio_buf:   Arc::new(Mutex::new(None)),
        stop_tx:     Arc::new(Mutex::new(None)),
        http_client: reqwest::Client::new(),
        db,
    };

    let _conn = dbus::build_connection(iface).await?;

    tracing::info!(
        "DBus service '{}' registered, waiting for commands",
        common::dbus::SERVICE_NAME
    );

    // Run forever — the connection is kept alive by holding `_conn`
    std::future::pending::<()>().await;
    Ok(())
}
