use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

use crate::bot::text::*;
use crate::db::session;
use crate::util::time::now_epoch;
use crate::web::WebState;

pub async fn app_page(
    State(state): State<WebState>,
    Path((chat_id, user_id, token)): Path<(i64, i64, String)>,
) -> Response {
    let now = now_epoch();

    let session = match session::find_active(&state.db, chat_id, user_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return page(StatusCode::NOT_FOUND, WEB_VERIFY_LINK_INVALID),
        Err(_) => return page(StatusCode::INTERNAL_SERVER_ERROR, WEB_VERIFY_SERVICE_ERROR),
    };
    if session.verify_token.as_deref() != Some(token.as_str()) {
        return page(StatusCode::NOT_FOUND, WEB_VERIFY_LINK_INVALID);
    }

    match session::mark_verified_if_pending_unexpired(&state.db, chat_id, user_id, &token, now)
        .await
    {
        Ok(true) => {}
        Ok(false) => return page(StatusCode::GONE, WEB_VERIFY_EXPIRED),
        Err(_) => return page(StatusCode::INTERNAL_SERVER_ERROR, WEB_VERIFY_SERVICE_ERROR),
    };

    if let Err(err) = state.telegram.unrestrict_member(chat_id, user_id).await {
        tracing::warn!(
            chat_id, user_id, error = %err, "unrestrict_member failed",
        );
        return page(
            StatusCode::INTERNAL_SERVER_ERROR,
            WEB_VERIFY_UNRESTRICT_FAILED,
        );
    }

    if let Some(msg_id) = session.verify_msg_id {
        let _ = state.telegram.delete_message(chat_id, msg_id).await;
    }
    page(StatusCode::OK, WEB_VERIFY_OK)
}

fn page(status: StatusCode, msg: &str) -> Response {
    (status, Html(render(msg))).into_response()
}

fn render(msg: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="zh">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{WEB_VERIFY_TITLE}</title>
<style>
body {{ font-family: -apple-system,BlinkMacSystemFont,sans-serif; padding: 24px; }}
main {{ text-align: center; margin-top: 48px; font-size: 17px; }}
</style>
</head>
<body>
<main>{msg}</main>
</body>
</html>"#
    )
}
