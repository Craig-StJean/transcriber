mod api;
mod audio;
mod audio_cues;
mod db;
mod dbus;
mod streaming;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("transcriber-daemon {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("transcriber-daemon starting");

    let cfg = common::config::load()?;
    tracing::info!("config loaded from {}", common::config::config_path().display());

    if cfg.active_key().is_empty() {
        tracing::warn!(
            "api_key is empty — transcription will fail until you set it in {}",
            common::config::config_path().display()
        );
    }

    let db = Arc::new(db::Database::open()?);

    // reqwest has no timeouts by default, so a stalled connection would leave
    // the daemon in Transcribing forever. 120 s covers uploading a
    // max-length recording on a slow uplink plus the provider's processing.
    let http_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()?;

    let iface = dbus::TranscriberInterface::new(cfg, http_client, db);

    let _conn = dbus::build_connection(iface).await?;

    tracing::info!(
        "DBus service '{}' registered, waiting for commands",
        common::dbus::SERVICE_NAME
    );

    // Run forever — the connection is kept alive by holding `_conn`
    std::future::pending::<()>().await;
    Ok(())
}
