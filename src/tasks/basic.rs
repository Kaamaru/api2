use axum::{
    extract::Json,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    routing::get,
    Json as AxumJson, Router,
};

use crate::{
    structs::{TheRequest, TheResponse},
    tasks::keycrypt::checkvalid,
};
use serde::{Deserialize, Serialize};

pub async fn handle_post(
    headers: HeaderMap,
    Json(payload): Json<TheRequest>,
) -> Result<AxumJson<TheResponse>, StatusCode> {
    println!("{}!", checkvalid(payload.text));

    for (key, value) in headers.iter() {
        println!("{}: {}", key, value.to_str().unwrap_or("invalid utf-8"));
    }
    if headers.get("RequestID").unwrap_or(
        &HeaderValue::from_str({
            println!("Nahhh");
            "Bruh Header ded"
        })
        .unwrap(),
    ) == "abc123"
    {
        println!("Sus!!")
    }

    Ok(AxumJson(TheResponse {
        text: "posted!".to_string(),
    }))
}

pub async fn handle_get() -> &'static str {
    "getty"
}
