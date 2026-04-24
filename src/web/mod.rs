use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;

use crate::bot::Bot;

pub mod pages;

pub type AppState = Arc<Bot>;
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/app/{chat_id}/{user_id}", get(pages::app_page))
        .with_state(state)
}
