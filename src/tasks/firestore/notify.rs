use crate::{
    structures::structs::{FirestoreUserResponse, Schedule},
    FirebaseConfig,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use reqwest::Client;
use serde_json::json;
use serde::Serialize;




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
        "{}/users/{}?updateMask.fieldPaths=schedules.{}.notify",
        firebase_config.firestore_url(),
        uuid,
        schedule_id
    );

    let mut payload = json!({
        "fields": {
            "schedules": { "mapValue": { "fields": {} } }
        }
    });

    // Dynamically set: fields.schedules.mapValue.fields.<schedule_id>.mapValue.fields.notify
    payload["fields"]["schedules"]["mapValue"]["fields"][schedule_id.clone()] = json!({
        "mapValue": {
            "fields": {
                "notify": { "booleanValue": value }
            }
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



// GET on: notify/get_notified/{uuid}
pub async fn firestore_get_notify_schedule_all(
    Path(uuid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let url = format!("{}/users/{}", firebase_config.firestore_url(), uuid);
    
    let resp = match reqwest::get(&url).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to call Firestore: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to connect to Firestore" })),
            )
                .into_response();
        }
    };

    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to read response body: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to read response" })),
            )
                .into_response();
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse Firestore response: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to parse Firestore data" })),
            )
                .into_response();
        }
    };

    let mut notified_schedules: Vec<Schedule> = Vec::new();

    // Handle single document response (has "fields" at top level)
    if let Some(fields) = parsed.get("fields").and_then(|f| f.as_object()) {
        if let Some(schedules) = fields.get("schedules") {
            if let Some(map_fields) = schedules.get("mapValue").and_then(|m| m.get("fields")).and_then(|f| f.as_object()) {
                for (_key, item) in map_fields {
                    if let Some(detail) = item.get("mapValue").and_then(|m| m.get("fields")).and_then(|f| f.as_object()) {
                        let notify = detail
                            .get("notify")
                            .and_then(|n| n.get("booleanValue"))
                            .and_then(|b| b.as_bool())
                            .unwrap_or(false);

                        if notify {
                            let name = detail.get("name").and_then(|n| n.get("stringValue")).and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let time = detail.get("time").and_then(|t| t.get("stringValue")).and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let date = detail.get("date").and_then(|d| d.get("stringValue")).and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let expiry = detail.get("expiry").and_then(|e| e.get("stringValue")).and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let uid = detail.get("uid").and_then(|u| u.get("stringValue")).and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let category = detail.get("category").and_then(|c| c.get("stringValue")).and_then(|s| s.as_str()).map(|s| s.to_string());

                            notified_schedules.push(Schedule {
                                name,
                                time,
                                date,
                                expiry,
                                uid,
                                notify,
                                category,
                            });
                        }
                    }
                }
            }
        }
    }

    (StatusCode::OK, Json(json!({
        "schedules": notified_schedules,
        "count": notified_schedules.len()
    }))).into_response()
}
// Regular function (non-axum) to get notified users and their schedules that expire within 3 days
pub async fn firestore_get_notified_users_schedules() -> Result<Vec<serde_json::Value>, String> {
    let url = format!("{}/users", crate::firebase::FirebaseConfig::from_env().firestore_url());

    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to call Firestore: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let parsed: FirestoreUserResponse = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Firestore response: {}", e))?;

    let mut users_with_notified: Vec<serde_json::Value> = Vec::new();
    let now = chrono::Utc::now();

    for doc in parsed.documents {
        if let Some(fields) = doc.fields {
            // Build list of this user's notified schedules that expire within 3 days
            let mut schedule_list: Vec<Schedule> = Vec::new();
            if let Some(schedules) = fields.schedules {
                for (_key, item) in schedules.map_value.fields {
                    let detail = item.map_value.fields;

                    let notify = detail
                        .notify
                        .as_ref()
                        .map(|n| n.booleanValue)
                        .unwrap_or(false);

                    // Only include if notify is true
                    if notify {
                        // Check if expiry is within 3 days
                        let expiry_str = detail.expiry
                            .as_ref()
                            .map(|e| e.stringValue.clone())
                            .unwrap_or_default();

                        // Parse expiry date in multiple formats
                        let is_expiring_soon = if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&expiry_str) {
                            let expiry_date = date.with_timezone(&chrono::Utc);
                            let time_until_expiry = expiry_date - now;
                            time_until_expiry.num_days() <= 3 && time_until_expiry.num_seconds() >= 0
                        } else if let Ok(date) = chrono::NaiveDate::parse_from_str(&expiry_str, "%Y-%m-%d") {
                            let datetime = chrono::DateTime::<chrono::Utc>::from_utc(
                                chrono::NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
                                chrono::Utc
                            );
                            let time_until_expiry = datetime - now;
                            time_until_expiry.num_days() <= 3 && time_until_expiry.num_seconds() >= 0
                        } else {
                            false
                        };

                        if is_expiring_soon {
                            schedule_list.push(Schedule {
                                name: detail.name
                                    .as_ref()
                                    .map(|n| n.stringValue.clone())
                                    .unwrap_or_default(),
                                time: detail.time
                                    .as_ref()
                                    .map(|t| t.stringValue.clone())
                                    .unwrap_or_default(),
                                date: detail.date
                                    .as_ref()
                                    .map(|d| d.stringValue.clone())
                                    .unwrap_or_default(),
                                expiry: expiry_str,
                                uid: detail.uid
                                    .as_ref()
                                    .map(|u| u.stringValue.clone())
                                    .unwrap_or_default(),
                                notify,
                                category: detail.category.as_ref().map(|c| c.stringValue.clone()),
                            });
                        }
                    }
                }
            }

            if !schedule_list.is_empty() {
                // Prefer email as user_id, else fallback to username
                let user_id = if let Some(email) = &fields.email {
                    email.stringValue.clone()
                } else if let Some(username) = &fields.username {
                    username.stringValue.clone()
                } else {
                    // If no identifier present, skip this user
                    continue;
                };

                users_with_notified.push(json!({
                    "user_id": user_id,
                    "schedules": schedule_list,
                }));
            }
        }
    }

    Ok(users_with_notified)
}