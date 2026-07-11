use crate::{structures::structs::Schedule, FirebaseConfig};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::TimeZone;
use reqwest::Client;
use serde_json::{json, Value};




// PATCH on: /notify/on/{uuid}/{schedule_id}
pub async fn firestore_set_notify_on(
    Path((uuid, schedule_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    set_notify_value(firebase_config, uuid, schedule_id, true).await
}

// PATCH on: /notify/off/{uuid}/{schedule_id}
pub async fn firestore_set_notify_off(
    Path((uuid, schedule_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    set_notify_value(firebase_config, uuid, schedule_id, false).await
}

async fn set_notify_value(
    firebase_config: FirebaseConfig,
    uuid: String,
    schedule_id: String,
    value: bool,
) -> impl IntoResponse {
    let client = Client::new();
    let patch_url = format!(
        "{}/users/{}/schedules/{}?updateMask.fieldPaths=notify",
        firebase_config.firestore_url(),
        uuid,
        schedule_id
    );

    let payload = json!({
        "fields": {
            "notify": { "booleanValue": value }
        }
    });

    let resp = match client.patch(&patch_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to PATCH Firestore: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to update notify value" })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Schedule not found" })),
            )
                .into_response();
        }
        let body = resp.text().await.unwrap_or_else(|_| "".to_string());
        eprintln!("Firestore PATCH failed: {} - {}", status, body);
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "Firestore update failed", "status": status.as_u16() })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "uuid": uuid,
            "schedule_id": schedule_id,
            "notify": value
        })),
    )
        .into_response()
}

fn extract_doc_id(name: &str) -> String {
    name.rsplit('/').next().unwrap_or_default().to_string()
}

fn parse_schedule_doc(schedule_doc: &Value) -> Schedule {
    let get_s = |k: &str| {
        schedule_doc
            .get("fields")
            .and_then(|f| f.get(k))
            .and_then(|v| v.get("stringValue"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
    };

    let get_b = |k: &str| {
        schedule_doc
            .get("fields")
            .and_then(|f| f.get(k))
            .and_then(|v| v.get("booleanValue"))
            .and_then(|b| b.as_bool())
    };

    let uid = get_s("uid").unwrap_or_else(|| {
        schedule_doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(extract_doc_id)
            .unwrap_or_default()
    });

    Schedule {
        name: get_s("name").unwrap_or_default(),
        time: get_s("time").unwrap_or_default(),
        date: get_s("date").unwrap_or_default(),
        expiry: get_s("expiry").unwrap_or_default(),
        uid,
        notify: get_b("notify").unwrap_or(false),
        category: get_s("category"),
        quantity: get_s("quantity"),
        location: get_s("location"),
    }
}

async fn fetch_user_schedules(
    client: &Client,
    firebase_config: &FirebaseConfig,
    uuid: &str,
) -> Result<Vec<Schedule>, String> {
    let url = format!("{}/users/{}/schedules", firebase_config.firestore_url(), uuid);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to call Firestore: {}", e))?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Firestore read failed {}: {}", status, body));
    }

    let parsed: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Firestore response: {}", e))?;

    let docs: Vec<Value> = parsed
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(docs.iter().map(parse_schedule_doc).collect())
}



// GET on: notify/get_notified/{uuid}
pub async fn firestore_get_notify_schedule_all(
    Path(uuid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let schedules = match fetch_user_schedules(&client, &firebase_config, &uuid).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to call Firestore: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to connect to Firestore" })),
            )
                .into_response();
        }
    };

    let notified_schedules: Vec<Schedule> = schedules.into_iter().filter(|s| s.notify).collect();

    (StatusCode::OK, Json(json!({
        "schedules": notified_schedules,
        "count": notified_schedules.len()
    }))).into_response()
}
// Regular function (non-axum) to get notified users and their schedules that expire within 3 days
pub async fn firestore_get_notified_users_schedules() -> Result<Vec<serde_json::Value>, String> {
    let firebase_config = crate::firebase::FirebaseConfig::from_env();
    let url = format!("{}/users", firebase_config.firestore_url());
    let client = Client::new();

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to call Firestore: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Firestore response: {}", e))?;

    let user_docs: Vec<Value> = parsed
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let mut users_with_notified: Vec<serde_json::Value> = Vec::new();
    let now = chrono::Utc::now();

    for user_doc in user_docs {
        let user_name = user_doc
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or_default();
        let uid = extract_doc_id(user_name);
        if uid.is_empty() {
            continue;
        }

        let user_identifier = user_doc
            .get("fields")
            .and_then(|f| f.get("email"))
            .and_then(|v| v.get("stringValue"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                user_doc
                    .get("fields")
                    .and_then(|f| f.get("username"))
                    .and_then(|v| v.get("stringValue"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            });

        let user_id = match user_identifier {
            Some(v) => v,
            None => continue,
        };

        // LINE user id (if the user has linked their account) for push notifications.
        let line_user_id = user_doc
            .get("fields")
            .and_then(|f| f.get("line_user_id"))
            .and_then(|v| v.get("stringValue"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let schedules = fetch_user_schedules(&client, &firebase_config, &uid).await?;
        let mut schedule_list: Vec<Schedule> = Vec::new();

        for schedule in schedules.into_iter().filter(|s| s.notify) {
            let expiry_str = schedule.expiry.clone();
            let is_expiring_soon = if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&expiry_str) {
                let expiry_date = date.with_timezone(&chrono::Utc);
                let time_until_expiry = expiry_date - now;
                time_until_expiry.num_days() <= 3 && time_until_expiry.num_seconds() >= 0
            } else if let Ok(date) = chrono::NaiveDate::parse_from_str(&expiry_str, "%Y-%m-%d") {
                let datetime = chrono::Utc.from_utc_datetime(
                    &chrono::NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                );
                let time_until_expiry = datetime - now;
                time_until_expiry.num_days() <= 3 && time_until_expiry.num_seconds() >= 0
            } else {
                false
            };

            if is_expiring_soon {
                schedule_list.push(schedule);
            }
        }

        if !schedule_list.is_empty() {
            users_with_notified.push(json!({
                "user_id": user_id,
                "uid": uid,
                "line_user_id": line_user_id,
                "schedules": schedule_list,
            }));
        }
    }

    Ok(users_with_notified)
}

/// Parse a schedule expiry string (RFC3339 or YYYY-MM-DD) into days-from-now.
fn days_until_expiry(expiry: &str) -> Option<i64> {
    let now = chrono::Utc::now();
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(expiry) {
        Some((date.with_timezone(&chrono::Utc) - now).num_days())
    } else if let Ok(date) = chrono::NaiveDate::parse_from_str(expiry, "%Y-%m-%d") {
        let dt = chrono::Utc.from_utc_datetime(&chrono::NaiveDateTime::new(
            date,
            chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
        ));
        Some((dt - now).num_days())
    } else {
        None
    }
}

/// POST /notify/test/{uid}
///
/// Immediately sends a notification to the given user via every channel they
/// have configured (email + LINE), using their soon-to-expire items. If they
/// have no expiring items, a synthetic test item is sent so delivery can still
/// be verified. Intended for manual testing from the app.
pub async fn firestore_test_notification(
    Path(uid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();

    // Resolve the user document for email + line_user_id.
    let user_url = format!("{}/users/{}", firebase_config.firestore_url(), uid);
    let user_doc: Value = match client.get(&user_url).send().await {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_else(|_| json!({})),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "User not found" })),
            )
                .into_response();
        }
    };

    let get_field = |key: &str| {
        user_doc
            .get("fields")
            .and_then(|f| f.get(key))
            .and_then(|v| v.get("stringValue"))
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string()
    };
    let email = get_field("email");
    let line_user_id = get_field("line_user_id");
    let any_channel = !email.is_empty() || !line_user_id.is_empty();

    // Collect the user's items expiring within a week (any notify flag), so the
    // test reflects real content when available.
    let schedules = fetch_user_schedules(&client, &firebase_config, &uid)
        .await
        .unwrap_or_default();

    let mut items: Vec<Value> = schedules
        .iter()
        .filter(|s| {
            days_until_expiry(&s.expiry)
                .map(|d| d >= 0 && d <= 7)
                .unwrap_or(false)
        })
        .filter_map(|s| serde_json::to_value(s).ok())
        .collect();

    let used_test_item = items.is_empty();
    if used_test_item {
        items.push(json!({
            "name": "CookFetch test item",
            "expiry": chrono::Utc::now().format("%Y-%m-%d").to_string(),
            "category": "Test",
            "date": "",
            "time": "",
        }));
    }

    // Email channel.
    let email_result = if email.is_empty() {
        json!({ "attempted": false, "reason": "no email on account" })
    } else {
        crate::tasks::sendmail::send_mail(&email, &items).await;
        json!({ "attempted": true, "to": email })
    };

    // LINE channel.
    let line_result = if line_user_id.is_empty() {
        json!({ "attempted": false, "reason": "LINE not linked" })
    } else {
        match crate::tasks::line::notify_line_expiring(&line_user_id, &items).await {
            Ok(()) => json!({ "attempted": true, "ok": true }),
            Err(e) => json!({ "attempted": true, "ok": false, "error": e }),
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "item_count": items.len(),
            "used_test_item": used_test_item,
            "any_channel": any_channel,
            "email": email_result,
            "line": line_result,
        })),
    )
        .into_response()
}