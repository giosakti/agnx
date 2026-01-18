use axum::http::StatusCode;

pub async fn livez() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

pub async fn readyz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}
