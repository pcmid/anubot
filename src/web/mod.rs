use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;

use crate::app::AppState;

pub mod pages;

pub type WebState = Arc<AppState>;
pub fn router(state: WebState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/metrics", get(metrics))
        .route("/app/{chat_id}/{user_id}/{token}", get(pages::app_page))
        .with_state(state)
}

async fn metrics(State(state): State<WebState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render_prometheus(),
    )
}
