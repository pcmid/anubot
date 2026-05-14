mod app;
mod bot;
mod config;
mod db;
mod metrics;
mod telegram;
mod util;
mod web;

use std::sync::Arc;

use config::Config;
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

    let state = Arc::new(app::AppState::new(db, &cfg).await?);

    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!(listen = %cfg.listen_addr, "web server listening");

    bot::routes::run_dispatcher(state.clone()).await;
    axum::serve(listener, web::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
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
