mod bot;
mod config;
mod db;
mod util;
mod web;

use std::sync::Arc;
use std::time::Duration;

use config::Config;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    let db = db::connect(&cfg.database_url).await?;

    let bot = Arc::new(bot::Bot::new(db, &cfg).await?);

    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!(listen = %cfg.listen_addr, "web server listening");

    let bot_dispatcher = bot.clone().run().await;
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let shutdown_task = tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });

    let web_shutdown = wait_for_shutdown(shutdown_rx.clone());
    axum::serve(listener, web::router(bot))
        .with_graceful_shutdown(web_shutdown)
        .await?;

    if !*shutdown_rx.borrow() {
        shutdown_task.abort();
    }

    match tokio::time::timeout(Duration::from_secs(5), bot_dispatcher.shutdown()).await {
        Ok(()) => {}
        Err(_) => tracing::warn!("dispatcher shutdown timed out; exiting process"),
    }

    Ok(())
}

async fn wait_for_shutdown(mut shutdown_rx: watch::Receiver<bool>) {
    if *shutdown_rx.borrow() {
        return;
    }

    while shutdown_rx.changed().await.is_ok() {
        if *shutdown_rx.borrow() {
            return;
        }
    }
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => tracing::info!("SIGINT received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
