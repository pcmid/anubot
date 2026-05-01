use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

use crate::bot::text::*;
use crate::db::session::{self, SessionStatus};
use crate::util::time::now_epoch;
use crate::web::AppState;

pub async fn app_page(
    State(bot): State<AppState>,
    Path((chat_id, user_id)): Path<(i64, i64)>,
) -> Response {
    let now = now_epoch();

    let session = match session::find_active(bot.db(), chat_id, user_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return page(StatusCode::NOT_FOUND, WEB_VERIFY_LINK_INVALID),
        Err(_) => return page(StatusCode::INTERNAL_SERVER_ERROR, WEB_VERIFY_SERVICE_ERROR),
    };
    if session.status != SessionStatus::Pending || session.expires_at <= now {
        return page(StatusCode::GONE, WEB_VERIFY_EXPIRED);
    }

    if let Err(err) = bot.unrestrict_member(chat_id, user_id).await {
        tracing::warn!(
            chat_id, user_id, error = %err, "unrestrict_member failed",
        );
        return page(
            StatusCode::INTERNAL_SERVER_ERROR,
            WEB_VERIFY_UNRESTRICT_FAILED,
        );
    }

    match session::mark_verified(bot.db(), chat_id, user_id, now).await {
        Ok(true) => {
            if let Some(msg_id) = session.verify_msg_id {
                let _ = bot.delete_message(chat_id, msg_id).await;
            }
            page(StatusCode::OK, WEB_VERIFY_OK)
        }
        Ok(false) => page(StatusCode::GONE, WEB_VERIFY_EXPIRED),
        Err(_) => page(StatusCode::INTERNAL_SERVER_ERROR, WEB_VERIFY_SERVICE_ERROR),
    }
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
