use axum::{
    extract::Json,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    routing::get,
    Json as AxumJson, Router,
};

use crate::{
    structures::structs::{TheRequest, TheResponse},
    tools::keycrypt::checkvalid,
};
use serde::{Deserialize, Serialize};

pub async fn handle_post(
    _headers: HeaderMap,
    Json(payload): Json<TheRequest>,
) -> Result<AxumJson<TheResponse>, StatusCode> {
    let _ = checkvalid(payload.text);

    Ok(AxumJson(TheResponse {
        text: "posted!".to_string(),
    }))
}

pub async fn handle_get() -> &'static str {
    "getty"
}
