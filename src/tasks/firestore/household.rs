// New household endpoints with refactored Firestore structure
// Structure: /households/{household_id}
// Fields: host (uid), name, members (array), invite_code, invite_expires_at

use crate::FirebaseConfig;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

fn generate_unique_id(prefix: &str) -> String {
    let now = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let uuid = Uuid::new_v4().simple();
    format!("{}_{}_{}", prefix, uuid, now)
}

fn extract_document_id(document_name: &str) -> String {
    document_name
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn get_doc_string_field(doc: &Value, key: &str) -> Option<String> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("stringValue"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

fn get_doc_array_field(doc: &Value, key: &str) -> Vec<String> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("arrayValue"))
        .and_then(|a| a.get("values"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.get("stringValue")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Deserialize)]
pub struct CreateHouseholdRequest {
    pub owner_uid: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct GenerateInviteRequest {
    pub actor_uid: String,
}

#[derive(Deserialize)]
pub struct RedeemInviteRequest {
    pub invite_code: String,
    pub member_uid: String,
}

#[derive(Deserialize)]
pub struct RemoveMemberRequest {
    pub member_uid: String,
}

#[derive(Deserialize)]
pub struct LeaveHouseholdRequest {
    pub member_uid: String,
}

// ============ CREATE HOUSEHOLD ============

pub async fn create_household(
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<CreateHouseholdRequest>,
) -> impl IntoResponse {
    if request.owner_uid.trim().is_empty() || request.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "owner_uid and name are required"})),
        )
            .into_response();
    }

    let household_id = generate_unique_id("hh");
    let client = Client::new();

    let household_url = format!(
        "{}/households?documentId={}",
        firebase_config.firestore_url(),
        household_id
    );

    let household_payload = json!({
        "fields": {
            "host": {"stringValue": request.owner_uid},
            "name": {"stringValue": request.name},
            "members": {"arrayValue": {"values": []}},
            "invite_code": {"nullValue": null},
            "invite_expires_at": {"nullValue": null}
        }
    });

    let household_resp = match client.post(&household_url).json(&household_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create household: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create household"})),
            )
                .into_response();
        }
    };

    if !household_resp.status().is_success() {
        let body = household_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create household", "details": body})),
        )
            .into_response();
    }

    // Store household_id on owner's user doc
    let user_url = format!(
        "{}/users/{}?updateMask.fieldPaths=household_id",
        firebase_config.firestore_url(),
        request.owner_uid
    );

    let user_payload = json!({
        "fields": {
            "household_id": {"stringValue": household_id}
        }
    });

    let _ = client.patch(&user_url).json(&user_payload).send().await;

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "household_id": household_id,
            "name": request.name,
            "owner_uid": request.owner_uid
        })),
    )
        .into_response()
}

// ============ GET MY HOUSEHOLD ============

pub async fn get_my_household(
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let member_uid = match query.get("member_uid") {
        Some(uid) if !uid.trim().is_empty() => uid.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "member_uid query parameter is required"})),
            )
                .into_response();
        }
    };

    let client = Client::new();

    // Get household_id from user doc
    let user_url = format!("{}/users/{}", firebase_config.firestore_url(), member_uid);

    let user_resp = match client.get(&user_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch user document"})),
            )
                .into_response();
        }
    };

    if !user_resp.status().is_success() {
        if user_resp.status() == StatusCode::NOT_FOUND {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "User not found"})),
            )
                .into_response();
        }

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch user document"})),
        )
            .into_response();
    }

    let user_doc: Value = match user_resp.json().await {
        Ok(doc) => doc,
        Err(_) => {
            return (
                StatusCode::OK,
                Json(json!({"household_id": null})),
            )
                .into_response();
        }
    };

    let household_id = match get_doc_string_field(&user_doc, "household_id") {
        Some(id) if !id.is_empty() => id,
        _ => {
            return (
                StatusCode::OK,
                Json(json!({"household_id": null})),
            )
                .into_response();
        }
    };

    // Fetch household doc
    let household_url = format!(
        "{}/households/{}",
        firebase_config.firestore_url(),
        household_id
    );

    let household_resp = match client.get(&household_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch household"})),
            )
                .into_response();
        }
    };

    if !household_resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Household not found"})),
        )
            .into_response();
    }

    let household_doc: Value = match household_resp.json().await {
        Ok(doc) => doc,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse household"})),
            )
                .into_response();
        }
    };

    let host = get_doc_string_field(&household_doc, "host").unwrap_or_default();
    let name = get_doc_string_field(&household_doc, "name").unwrap_or_default();
    let members = get_doc_array_field(&household_doc, "members");
    let is_host = member_uid == host;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "household_id": household_id,
            "name": name,
            "host": host,
            "members": members,
            "is_host": is_host
        })),
    )
        .into_response()
}

// ============ GENERATE INVITE CODE ============

const INVITE_CODE_PREFIX: &str = "HH";
const INVITE_EXPIRY_HOURS: i64 = 24;

fn generate_invite_code() -> String {
    let uuid_part = Uuid::new_v4().simple().to_string();
    let short_code = &uuid_part[0..6].to_uppercase();
    format!("{}-{}", INVITE_CODE_PREFIX, short_code)
}

pub async fn generate_household_invite(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<GenerateInviteRequest>,
) -> impl IntoResponse {
    if request.actor_uid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "actor_uid is required"})),
        )
            .into_response();
    }

    let client = Client::new();

    // Fetch household to verify actor is host
    let household_url = format!(
        "{}/households/{}",
        firebase_config.firestore_url(),
        household_id
    );

    let household_resp = match client.get(&household_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch household"})),
            )
                .into_response();
        }
    };

    if !household_resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Household not found"})),
        )
            .into_response();
    }

    let household_doc: Value = match household_resp.json().await {
        Ok(doc) => doc,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse household"})),
            )
                .into_response();
        }
    };

    let host = get_doc_string_field(&household_doc, "host").unwrap_or_default();
    if host != request.actor_uid {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only host can generate invites"})),
        )
            .into_response();
    }

    // Generate code and set on household doc
    let invite_code = generate_invite_code();
    let now = Utc::now();
    let expires_at = now + Duration::hours(INVITE_EXPIRY_HOURS);

    let update_url = format!(
        "{}?updateMask.fieldPaths=invite_code&updateMask.fieldPaths=invite_expires_at",
        household_url
    );

    let update_payload = json!({
        "fields": {
            "invite_code": {"stringValue": invite_code.clone()},
            "invite_expires_at": {"stringValue": expires_at.to_rfc3339()}
        }
    });

    let update_resp = match client.patch(&update_url).json(&update_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to set invite code", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !update_resp.status().is_success() {
        let body = update_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to set invite code", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "invite_code": invite_code,
            "household_id": household_id,
            "expires_at": expires_at.to_rfc3339()
        })),
    )
        .into_response()
}

// ============ REDEEM INVITE CODE ============

pub async fn redeem_household_invite(
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<RedeemInviteRequest>,
) -> impl IntoResponse {
    if request.invite_code.trim().is_empty() || request.member_uid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invite_code and member_uid are required"})),
        )
            .into_response();
    }

    let client = Client::new();

    // Search for household with matching invite code
    let households_url = format!("{}/households", firebase_config.firestore_url());

    let households_resp = match client.get(&households_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch households"})),
            )
                .into_response();
        }
    };

    if !households_resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Household not found"})),
        )
            .into_response();
    }

    let households_data: Value = match households_resp.json().await {
        Ok(data) => data,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse households"})),
            )
                .into_response();
        }
    };

    // Find matching household
    let documents = households_data
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let mut matching_household: Option<(String, Value)> = None;

    for doc in documents {
        if let Some(stored_code) = get_doc_string_field(&doc, "invite_code") {
            if stored_code == request.invite_code {
                if let Some(name) = doc.get("name").and_then(|n| n.as_str()) {
                    let household_id = extract_document_id(name);
                    matching_household = Some((household_id, doc.clone()));
                }
                break;
            }
        }
    }

    let (household_id, household_doc) = match matching_household {
        Some(hh) => hh,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Invite code not found"})),
            )
                .into_response();
        }
    };

    // Verify not expired
    if let Some(expires_at_str) = get_doc_string_field(&household_doc, "invite_expires_at") {
        if let Ok(expires_at) = expires_at_str.parse::<chrono::DateTime<chrono::Utc>>() {
            if Utc::now() > expires_at {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Invite code has expired"})),
                )
                    .into_response();
            }
        }
    }

    // Check if already in members
    let members = get_doc_array_field(&household_doc, "members");
    if members.contains(&request.member_uid) {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "Member already in household"})),
        )
            .into_response();
    }

    // Add to members array
    let mut new_members = members.clone();
    new_members.push(request.member_uid.clone());

    let household_url = format!(
        "{}/households/{}?updateMask.fieldPaths=members&updateMask.fieldPaths=invite_code&updateMask.fieldPaths=invite_expires_at",
        firebase_config.firestore_url(),
        household_id
    );

    let members_values: Vec<Value> = new_members
        .iter()
        .map(|m| json!({"stringValue": m}))
        .collect();

    let update_payload = json!({
        "fields": {
            "members": {"arrayValue": {"values": members_values}},
            "invite_code": {"nullValue": null},
            "invite_expires_at": {"nullValue": null}
        }
    });

    let update_resp = match client.patch(&household_url).json(&update_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to add member", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !update_resp.status().is_success() {
        let body = update_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to add member", "details": body})),
        )
            .into_response();
    }

    // Store household_id on member's user doc
    let user_url = format!(
        "{}/users/{}?updateMask.fieldPaths=household_id",
        firebase_config.firestore_url(),
        request.member_uid
    );

    let user_payload = json!({
        "fields": {
            "household_id": {"stringValue": household_id.clone()}
        }
    });

    let _ = client.patch(&user_url).json(&user_payload).send().await;

    let household_name = get_doc_string_field(&household_doc, "name").unwrap_or_default();

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "household_id": household_id,
            "name": household_name,
            "member_uid": request.member_uid,
            "message": "Successfully joined household!"
        })),
    )
        .into_response()
}

// ============ REMOVE MEMBER (HOST ONLY) ============

pub async fn remove_member(
    Path((household_id, member_uid)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let actor_uid = match query.get("actor_uid") {
        Some(uid) if !uid.trim().is_empty() => uid.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "actor_uid query parameter is required"})),
            )
                .into_response();
        }
    };

    let client = Client::new();

    // Fetch household to verify actor is host
    let household_url = format!(
        "{}/households/{}",
        firebase_config.firestore_url(),
        household_id
    );

    let household_resp = match client.get(&household_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch household"})),
            )
                .into_response();
        }
    };

    if !household_resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Household not found"})),
        )
            .into_response();
    }

    let household_doc: Value = match household_resp.json().await {
        Ok(doc) => doc,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse household"})),
            )
                .into_response();
        }
    };

    let host = get_doc_string_field(&household_doc, "host").unwrap_or_default();
    if host != actor_uid {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only host can remove members"})),
        )
            .into_response();
    }

    // Cannot remove the host
    if member_uid == host {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Cannot remove the host"})),
        )
            .into_response();
    }

    // Remove from members array
    let members = get_doc_array_field(&household_doc, "members");
    let new_members: Vec<String> = members.into_iter().filter(|m| m != &member_uid).collect();

    let update_url = format!(
        "{}?updateMask.fieldPaths=members",
        household_url
    );

    let members_values: Vec<Value> = new_members
        .iter()
        .map(|m| json!({"stringValue": m}))
        .collect();

    let update_payload = json!({
        "fields": {
            "members": {"arrayValue": {"values": members_values}}
        }
    });

    let update_resp = match client.patch(&update_url).json(&update_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to remove member", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !update_resp.status().is_success() {
        let body = update_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to remove member", "details": body})),
        )
            .into_response();
    }

    // Clear household_id from member's user doc
    let user_url = format!(
        "{}/users/{}?updateMask.fieldPaths=household_id",
        firebase_config.firestore_url(),
        member_uid
    );

    let user_payload = json!({
        "fields": {
            "household_id": {"nullValue": null}
        }
    });

    let _ = client.patch(&user_url).json(&user_payload).send().await;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Member removed"
        })),
    )
        .into_response()
}

// ============ LEAVE HOUSEHOLD (MEMBER SELF-REMOVE) ============

pub async fn leave_household(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<LeaveHouseholdRequest>,
) -> impl IntoResponse {
    let client = Client::new();

    // Fetch household
    let household_url = format!(
        "{}/households/{}",
        firebase_config.firestore_url(),
        household_id
    );

    let household_resp = match client.get(&household_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch household"})),
            )
                .into_response();
        }
    };

    if !household_resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Household not found"})),
        )
            .into_response();
    }

    let household_doc: Value = match household_resp.json().await {
        Ok(doc) => doc,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse household"})),
            )
                .into_response();
        }
    };

    let host = get_doc_string_field(&household_doc, "host").unwrap_or_default();

    // Host cannot leave
    if request.member_uid == host {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Host cannot leave household"})),
        )
            .into_response();
    }

    // Remove from members array
    let members = get_doc_array_field(&household_doc, "members");
    let new_members: Vec<String> = members.into_iter().filter(|m| m != &request.member_uid).collect();

    let update_url = format!(
        "{}?updateMask.fieldPaths=members",
        household_url
    );

    let members_values: Vec<Value> = new_members
        .iter()
        .map(|m| json!({"stringValue": m}))
        .collect();

    let update_payload = json!({
        "fields": {
            "members": {"arrayValue": {"values": members_values}}
        }
    });

    let update_resp = match client.patch(&update_url).json(&update_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to leave household", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !update_resp.status().is_success() {
        let body = update_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to leave household", "details": body})),
        )
            .into_response();
    }

    // Clear household_id from member's user doc
    let user_url = format!(
        "{}/users/{}?updateMask.fieldPaths=household_id",
        firebase_config.firestore_url(),
        request.member_uid
    );

    let user_payload = json!({
        "fields": {
            "household_id": {"nullValue": null}
        }
    });

    let _ = client.patch(&user_url).json(&user_payload).send().await;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Successfully left household"
        })),
    )
        .into_response()
}

// ============ HELPER: VERIFY MEMBER ACCESS ============

pub async fn is_member_or_host(
    client: &Client,
    firebase_config: &FirebaseConfig,
    household_id: &str,
    member_uid: &str,
) -> Result<bool, String> {
    let household_url = format!(
        "{}/households/{}",
        firebase_config.firestore_url(),
        household_id
    );

    let resp = client
        .get(&household_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Ok(false);
    }

    let doc: Value = resp.json().await.map_err(|e| e.to_string())?;

    let host = get_doc_string_field(&doc, "host").unwrap_or_default();
    if member_uid == host {
        return Ok(true);
    }

    let members = get_doc_array_field(&doc, "members");
    Ok(members.contains(&member_uid.to_string()))
}
