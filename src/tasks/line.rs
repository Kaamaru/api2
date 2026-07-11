//! LINE Messaging API integration.
//!
//! - Account linking: a logged-in user generates a short-lived 6-digit code,
//!   adds the LINE Official Account as a friend, and sends the code. The webhook
//!   matches the code and records `line_user_id` on the user document.
//! - Push notifications: `push_line_message` implements the retry/idempotency
//!   rules required by the LINE push endpoint.
//!
//! Secrets (`LINE_CHANNEL_SECRET`, `LINE_CHANNEL_ACCESS_TOKEN`) are read from the
//! environment on demand and are never logged.

use crate::firebase::FirebaseConfig;
use axum::{
    body::Bytes,
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration as ChronoDuration, Utc};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;
use std::time::Duration;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const LINE_PUSH_URL: &str = "https://api.line.me/v2/bot/message/push";
/// Codes are valid for 10 minutes.
const LINK_CODE_TTL_MINUTES: i64 = 10;
/// Push is retried at most this many times (only on 500 / timeout).
const PUSH_MAX_ATTEMPTS: u32 = 3;

fn line_channel_secret() -> Result<String, String> {
    std::env::var("LINE_CHANNEL_SECRET").map_err(|_| "LINE_CHANNEL_SECRET not set".to_string())
}

fn line_channel_access_token() -> Result<String, String> {
    std::env::var("LINE_CHANNEL_ACCESS_TOKEN")
        .map_err(|_| "LINE_CHANNEL_ACCESS_TOKEN not set".to_string())
}

fn extract_doc_id(name: &str) -> String {
    name.rsplit('/').next().unwrap_or_default().to_string()
}

fn get_doc_string_field(doc: &Value, key: &str) -> Option<String> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("stringValue"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

// ===================== Push notifications =====================

/// Push one or more LINE message objects to `to` (a LINE user id).
///
/// Idempotency & retry rules:
/// - A single `X-Line-Retry-Key` (UUID) is generated per logical send and reused
///   across all retries so LINE de-duplicates repeated attempts.
/// - Retries happen only on HTTP 500 or a request timeout, up to
///   `PUSH_MAX_ATTEMPTS`, with exponential backoff.
/// - HTTP 409 (conflict — the retry key was already accepted) is treated as
///   success.
/// - Any other 4xx is a permanent failure and is NOT retried.
pub async fn push_line_message(to: &str, messages: Vec<Value>) -> Result<(), String> {
    let access_token = line_channel_access_token()?;
    let retry_key = Uuid::new_v4().to_string();

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let payload = json!({ "to": to, "messages": messages });

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;

        let result = client
            .post(LINE_PUSH_URL)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("X-Line-Retry-Key", &retry_key)
            .json(&payload)
            .send()
            .await;

        match result {
            Ok(response) => {
                let status = response.status();

                if status.is_success() {
                    return Ok(());
                }

                // 409: the same retry key was already accepted — treat as success.
                if status == StatusCode::CONFLICT {
                    return Ok(());
                }

                // 500: retryable server error.
                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    if attempt >= PUSH_MAX_ATTEMPTS {
                        return Err(format!("LINE push failed after {attempt} attempts (500)"));
                    }
                    backoff(attempt).await;
                    continue;
                }

                // Any other status (other 4xx, etc.) — do not retry.
                let body = response.text().await.unwrap_or_default();
                return Err(format!("LINE push failed ({}): {}", status, body));
            }
            Err(err) => {
                // Retry only on timeout.
                if err.is_timeout() {
                    if attempt >= PUSH_MAX_ATTEMPTS {
                        return Err(format!("LINE push timed out after {attempt} attempts"));
                    }
                    backoff(attempt).await;
                    continue;
                }
                return Err(format!("LINE push request error: {err}"));
            }
        }
    }
}

/// Exponential backoff: 0.5s, 1s, 2s, ...
async fn backoff(attempt: u32) {
    let millis = 500u64 * 2u64.pow(attempt.saturating_sub(1));
    tokio::time::sleep(Duration::from_millis(millis)).await;
}

/// Convenience wrapper: send a single text message.
pub async fn push_line_text(to: &str, text: &str) -> Result<(), String> {
    push_line_message(to, vec![json!({ "type": "text", "text": text })]).await
}

/// Push an expiry-reminder summary for a user's soon-to-expire inventory items.
/// `items` are schedule JSON values (as produced by the notified-users query).
pub async fn notify_line_expiring(line_user_id: &str, items: &[Value]) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let mut text = String::from("🔔 CookFetch reminder — items expiring soon:\n");
    for item in items {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let expiry = item.get("expiry").and_then(|v| v.as_str()).unwrap_or("");
        let category = item
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if expiry.is_empty() {
            text.push_str(&format!("• {name}\n"));
        } else if category.is_empty() {
            text.push_str(&format!("• {name} (expires {expiry})\n"));
        } else {
            text.push_str(&format!("• {name} [{category}] — expires {expiry}\n"));
        }
    }
    text.push_str("\nUse them up or discard soon! 🍳");

    push_line_text(line_user_id, text.trim_end()).await
}

// ===================== Link code generation =====================

#[derive(Deserialize)]
pub struct LinkCodeRequest {
    pub userid: String,
}

/// Generate a random 6-digit code without pulling in an extra RNG crate:
/// derive it from a fresh UUID's entropy.
fn generate_six_digit_code() -> String {
    let n = (Uuid::new_v4().as_u128() % 1_000_000) as u32;
    format!("{:06}", n)
}

/// POST /line/link-code — body: { "userid": "<uid>" }.
/// Creates a `link_codes/{code}` document valid for 10 minutes and returns it.
pub async fn generate_link_code_handler(
    Json(request): Json<LinkCodeRequest>,
) -> impl IntoResponse {
    if request.userid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "userid is required" })),
        )
            .into_response();
    }

    let firebase_config = FirebaseConfig::from_env();
    let client = Client::new();

    let expires_at = Utc::now() + ChronoDuration::minutes(LINK_CODE_TTL_MINUTES);
    let expires_at_str = expires_at.to_rfc3339();

    // Try a few times in the unlikely event of a code collision.
    for _ in 0..5 {
        let code = generate_six_digit_code();
        let url = format!(
            "{}/link_codes?documentId={}",
            firebase_config.firestore_url(),
            code
        );

        let payload = json!({
            "fields": {
                "code": { "stringValue": code },
                "user_id": { "stringValue": request.userid },
                "expires_at": { "stringValue": expires_at_str }
            }
        });

        let resp = match client.post(&url).json(&payload).send().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("line: failed to create link code: {e}");
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": "Failed to create link code" })),
                )
                    .into_response();
            }
        };

        if resp.status().is_success() {
            return (
                StatusCode::OK,
                Json(json!({
                    "code": code,
                    "expires_at": expires_at_str,
                    "expires_in_minutes": LINK_CODE_TTL_MINUTES
                })),
            )
                .into_response();
        }

        // 409 ALREADY_EXISTS => collision, generate another code and retry.
        if resp.status() == StatusCode::CONFLICT {
            continue;
        }

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        eprintln!("line: link code create failed {status}: {body}");
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "Failed to create link code" })),
        )
            .into_response();
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "Could not allocate a unique link code" })),
    )
        .into_response()
}

/// GET /line/status/{userid} — reports whether the user has linked LINE.
pub async fn line_status_handler(Path(userid): Path<String>) -> impl IntoResponse {
    let firebase_config = FirebaseConfig::from_env();
    let client = Client::new();
    let url = format!("{}/users/{}", firebase_config.firestore_url(), userid);

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("line: failed to read user for status: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Failed to read user" })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        return (StatusCode::OK, Json(json!({ "linked": false }))).into_response();
    }

    let user_doc: Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return (StatusCode::OK, Json(json!({ "linked": false }))).into_response(),
    };

    let linked = get_doc_string_field(&user_doc, "line_user_id")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    (StatusCode::OK, Json(json!({ "linked": linked }))).into_response()
}

// ===================== Webhook =====================

/// Constant-time-ish comparison of the received signature against the expected one.
fn signatures_match(expected: &str, received: &str) -> bool {
    let a = expected.as_bytes();
    let b = received.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verify `x-line-signature` = base64(HMAC-SHA256(channel_secret, raw_body)).
fn verify_signature(raw_body: &[u8], headers: &HeaderMap, secret: &str) -> bool {
    let received = match headers.get("x-line-signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(raw_body);
    let expected = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    signatures_match(&expected, received)
}

/// POST /line/webhook.
///
/// The signature is verified against the RAW request body BEFORE the body is
/// parsed as JSON. On a text `message` event whose text matches an unexpired
/// link code, the sender's LINE user id is written onto the corresponding user
/// document and the code is deleted.
pub async fn line_webhook_handler(headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    let secret = match line_channel_secret() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("line webhook: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    // 1) Verify signature against the raw bytes first.
    if !verify_signature(&body, &headers, &secret) {
        eprintln!("line webhook: invalid signature");
        return StatusCode::UNAUTHORIZED;
    }

    // 2) Only now parse the body as JSON.
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("line webhook: invalid JSON body: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    let events = payload
        .get("events")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();

    let firebase_config = FirebaseConfig::from_env();
    let client = Client::new();

    for event in events {
        let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if event_type != "message" {
            continue;
        }

        let message_type = event
            .get("message")
            .and_then(|m| m.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        if message_type != "text" {
            continue;
        }

        let text = event
            .get("message")
            .and_then(|m| m.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        let line_user_id = event
            .get("source")
            .and_then(|s| s.get("userId"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();

        if text.is_empty() || line_user_id.is_empty() {
            continue;
        }

        try_link_code(&client, &firebase_config, &text, &line_user_id).await;
    }

    // LINE expects a 200 for every delivery.
    StatusCode::OK
}

/// Look up a link code; if valid and unexpired, bind `line_user_id` to the user
/// and delete the code. Best-effort — failures are logged, never surfaced to LINE.
async fn try_link_code(
    client: &Client,
    firebase_config: &FirebaseConfig,
    code: &str,
    line_user_id: &str,
) {
    // Only 6-digit numeric strings can be codes; ignore anything else quietly.
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return;
    }

    let code_url = format!("{}/link_codes/{}", firebase_config.firestore_url(), code);

    let resp = match client.get(&code_url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("line webhook: failed to read link code: {e}");
            return;
        }
    };

    if !resp.status().is_success() {
        // No such code (or expired/deleted). Nothing to do.
        return;
    }

    let code_doc: Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return,
    };

    let user_id = match get_doc_string_field(&code_doc, "user_id") {
        Some(id) if !id.trim().is_empty() => id,
        _ => return,
    };

    // Check expiry.
    if let Some(expires_at) = get_doc_string_field(&code_doc, "expires_at") {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(&expires_at) {
            if Utc::now() > expiry.with_timezone(&Utc) {
                // Expired: clean it up and stop.
                let _ = client.delete(&code_url).send().await;
                return;
            }
        }
    }

    // Write line_user_id onto the user document.
    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=line_user_id",
        firebase_config.firestore_url(),
        user_id
    );
    let patch_payload = json!({
        "fields": { "line_user_id": { "stringValue": line_user_id } }
    });

    match client.patch(&patch_url).json(&patch_payload).send().await {
        Ok(r) if r.status().is_success() => {
            // Delete the code so it can't be reused.
            let _ = client.delete(&code_url).send().await;
            eprintln!("line webhook: linked user {user_id} to LINE");

            // Best-effort confirmation message.
            let _ = push_line_text(
                line_user_id,
                "✅ Your CookFetch account is now linked! You'll get expiry reminders here.",
            )
            .await;
        }
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            eprintln!("line webhook: failed to set line_user_id {status}: {body}");
        }
        Err(e) => eprintln!("line webhook: failed to patch user: {e}"),
    }
}
