mod bot;
mod config;
mod db;
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

    let bot = Arc::new(bot::Bot::new(db, &cfg).await?);

    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!(listen = %cfg.listen_addr, "web server listening");

    {
        let bot = bot.clone();
        tokio::spawn(async move { bot.run().await });
    }

    axum::serve(listener, web::router(bot)).await?;
    Ok(())
}
