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
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let url = format!("{}/users", firebase_config.firestore_url());
    
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

    let parsed: FirestoreUserResponse = match serde_json::from_str(&text) {
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

    for doc in parsed.documents {
        if let Some(fields) = doc.fields {
            if let Some(schedules) = fields.schedules {
                for (_key, item) in schedules.map_value.fields {
                    let detail = item.map_value.fields;

                    // Filter for only schedules where notify is true
                    let notify = detail
                        .notify
                        .as_ref()
                        .map(|n| n.booleanValue)
                        .unwrap_or(false);

                    if notify {
                        notified_schedules.push(Schedule {
                            name: detail.name.stringValue.clone(),
                            time: detail.time.stringValue.clone(),
                            date: detail.date.stringValue.clone(),
                            expiry: detail.expiry.stringValue.clone(),
                            uid: detail.uid.stringValue.clone(),
                            notify,
                            category: detail.category.map(|c| c.stringValue),
                        });
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

// GET: /notify/get_notified_users
pub async fn firestore_get_notified_users_schedules(
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let url = format!("{}/users", firebase_config.firestore_url());

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

    let parsed: FirestoreUserResponse = match serde_json::from_str(&text) {
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

    let mut users_with_notified: Vec<serde_json::Value> = Vec::new();

    for doc in parsed.documents {
        if let Some(fields) = doc.fields {
            // Build list of this user's notified schedules
            let mut schedule_list: Vec<Schedule> = Vec::new();
            if let Some(schedules) = fields.schedules {
                for (_key, item) in schedules.map_value.fields {
                    let detail = item.map_value.fields;

                    let notify = detail
                        .notify
                        .as_ref()
                        .map(|n| n.booleanValue)
                        .unwrap_or(false);

                    if notify {
                        schedule_list.push(Schedule {
                            name: detail.name.stringValue.clone(),
                            time: detail.time.stringValue.clone(),
                            date: detail.date.stringValue.clone(),
                            expiry: detail.expiry.stringValue.clone(),
                            uid: detail.uid.stringValue.clone(),
                            notify,
                            category: detail.category.map(|c| c.stringValue),
                        });
                    }
                }
            }

            if !schedule_list.is_empty() {
                // Prefer email as user_id, else fallback to username
                let user_id = if let Some(email) = fields.email {
                    email.stringValue
                } else if let Some(username) = fields.username {
                    username.stringValue
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

    (StatusCode::OK, Json(json!({
        "users": users_with_notified,
        "count": users_with_notified.len()
    }))).into_response()
}