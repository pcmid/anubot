use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;

use crate::app::AppState;

pub mod pages;

pub type WebState = Arc<AppState>;
pub fn router(state: WebState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/app/{chat_id}/{user_id}/{token}", get(pages::app_page))
        .with_state(state)
}
