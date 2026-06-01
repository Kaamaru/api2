use crate::{
    structures::structs::{AddScheduleRequest, CreateGroceryListRequest},
    FirebaseConfig,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
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

#[derive(Deserialize)]
pub struct CreateHouseholdRequest {
    pub owner_uid: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct JoinHouseholdRequest {
    pub requester_uid: String,
    #[serde(default)]
    pub requested_role: Option<String>,
}

#[derive(Deserialize)]
pub struct ReviewJoinRequest {
    pub actor_uid: String,
    pub action: String,
}

#[derive(Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub actor_uid: String,
    pub role: String,
}

#[derive(Deserialize)]
pub struct RemoveMemberRequest {
    pub actor_uid: String,
}

#[derive(Deserialize)]
pub struct HouseholdAddScheduleRequest {
    pub member_uid: String,
    pub name: String,
    pub time: String,
    pub date: String,
    pub expiry: String,
    pub notify: bool,
    pub category: Option<String>,
    pub quantity: String,
    pub location: Option<String>,
}

#[derive(Deserialize)]
pub struct HouseholdCreateGroceryListRequest {
    pub member_uid: String,
    pub list_name: String,
}

async fn is_owner(
    client: &Client,
    firebase_config: &FirebaseConfig,
    household_id: &str,
    uid: &str,
) -> Result<bool, String> {
    let member_url = format!(
        "{}/households/{}/members/{}",
        firebase_config.firestore_url(),
        household_id,
        uid
    );

    let resp = client
        .get(&member_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch member: {e}"))?;

    if !resp.status().is_success() {
        return Ok(false);
    }

    let doc: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse member document: {e}"))?;

    let role = get_doc_string_field(&doc, "role").unwrap_or_default();
    let status = get_doc_string_field(&doc, "status").unwrap_or_default();

    Ok(role == "owner" && status == "approved")
}

async fn is_approved_member(
    client: &Client,
    firebase_config: &FirebaseConfig,
    household_id: &str,
    uid: &str,
) -> Result<bool, String> {
    let member_url = format!(
        "{}/households/{}/members/{}",
        firebase_config.firestore_url(),
        household_id,
        uid
    );

    let resp = client
        .get(&member_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch member: {e}"))?;

    if !resp.status().is_success() {
        return Ok(false);
    }

    let doc: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse member document: {e}"))?;

    let status = get_doc_string_field(&doc, "status").unwrap_or_default();
    Ok(status == "approved")
}

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

    let household_id = generate_unique_id("household");
    let now = Utc::now().to_rfc3339();
    let client = Client::new();

    let household_url = format!(
        "{}/households?documentId={}",
        firebase_config.firestore_url(),
        household_id
    );

    let household_payload = json!({
        "fields": {
            "name": {"stringValue": request.name},
            "owner_uid": {"stringValue": request.owner_uid},
            "created_at": {"stringValue": now.clone()}
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

    let owner_member_url = format!(
        "{}/households/{}/members?documentId={}",
        firebase_config.firestore_url(),
        household_id,
        request.owner_uid
    );

    let owner_member_payload = json!({
        "fields": {
            "uid": {"stringValue": request.owner_uid},
            "role": {"stringValue": "owner"},
            "status": {"stringValue": "approved"},
            "joined_at": {"stringValue": now}
        }
    });

    let owner_resp = match client
        .post(&owner_member_url)
        .json(&owner_member_payload)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create owner membership: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create owner membership"})),
            )
                .into_response();
        }
    };

    if !owner_resp.status().is_success() {
        let body = owner_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create owner membership", "details": body})),
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

    let user_resp = client.patch(&user_url).json(&user_payload).send().await;
    if let Err(e) = user_resp {
        eprintln!("Warning: Failed to store household_id on owner doc: {}", e);
    }

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

pub async fn get_household_summary(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let member_uid = query.get("member_uid").cloned().unwrap_or_default();
    if member_uid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "member_uid query is required"})),
        )
            .into_response();
    }

    let client = Client::new();
    let is_member = match is_approved_member(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership", "details": e})),
            )
                .into_response()
        }
    };

    if !is_member {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only approved members can view household"})),
        )
            .into_response();
    }

    let household_url = format!(
        "{}/households/{}",
        firebase_config.firestore_url(),
        household_id
    );

    let household_resp = match client.get(&household_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch household", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !household_resp.status().is_success() {
        let body = household_resp.text().await.unwrap_or_default();
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Household not found", "details": body})),
        )
            .into_response();
    }

    let household_doc: Value = match household_resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse household", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    let members_url = format!(
        "{}/households/{}/members",
        firebase_config.firestore_url(),
        household_id
    );

    let members_resp = match client.get(&members_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch members", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    let members_payload: Value = members_resp.json().await.unwrap_or_else(|_| json!({}));
    let member_docs = members_payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let members: Vec<Value> = member_docs
        .iter()
        .map(|doc| {
            json!({
                "uid": get_doc_string_field(doc, "uid").unwrap_or_else(|| doc.get("name").and_then(|n| n.as_str()).map(extract_document_id).unwrap_or_default()),
                "role": get_doc_string_field(doc, "role").unwrap_or_default(),
                "status": get_doc_string_field(doc, "status").unwrap_or_default(),
                "joined_at": get_doc_string_field(doc, "joined_at")
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "household": {
                "household_id": household_id,
                "name": get_doc_string_field(&household_doc, "name").unwrap_or_default(),
                "owner_uid": get_doc_string_field(&household_doc, "owner_uid").unwrap_or_default(),
                "created_at": get_doc_string_field(&household_doc, "created_at")
            },
            "members": members,
            "member_count": members.len()
        })),
    )
        .into_response()
}

pub async fn create_join_request(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<JoinHouseholdRequest>,
) -> impl IntoResponse {
    if request.requester_uid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "requester_uid is required"})),
        )
            .into_response();
    }

    let client = Client::new();

    let already_member = match is_approved_member(
        &client,
        &firebase_config,
        &household_id,
        &request.requester_uid,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership", "details": e})),
            )
                .into_response()
        }
    };

    if already_member {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "User is already an approved member"})),
        )
            .into_response();
    }

    let requests_url = format!(
        "{}/households/{}/join_requests",
        firebase_config.firestore_url(),
        household_id
    );

    let existing_requests_resp = match client.get(&requests_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to check existing requests", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if existing_requests_resp.status().is_success() {
        let payload: Value = existing_requests_resp.json().await.unwrap_or_else(|_| json!({}));
        let docs = payload
            .get("documents")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let duplicate_pending = docs.iter().any(|doc| {
            get_doc_string_field(doc, "requester_uid").as_deref() == Some(&request.requester_uid)
                && get_doc_string_field(doc, "status").as_deref() == Some("pending")
        });

        if duplicate_pending {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error": "Pending request already exists"})),
            )
                .into_response();
        }
    }

    let request_id = generate_unique_id("joinreq");
    let post_url = format!(
        "{}/households/{}/join_requests?documentId={}",
        firebase_config.firestore_url(),
        household_id,
        request_id
    );

    let payload = json!({
        "fields": {
            "requester_uid": {"stringValue": request.requester_uid},
            "requested_role": {"stringValue": request.requested_role.unwrap_or_else(|| "member".to_string())},
            "status": {"stringValue": "pending"},
            "created_at": {"stringValue": Utc::now().to_rfc3339()}
        }
    });

    let response = match client.post(&post_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create join request", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create join request", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "household_id": household_id,
            "request_id": request_id,
            "status": "pending"
        })),
    )
        .into_response()
}

pub async fn list_join_requests(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let actor_uid = query.get("actor_uid").cloned().unwrap_or_default();
    if actor_uid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "actor_uid query is required"})),
        )
            .into_response();
    }

    let client = Client::new();
    let owner = match is_owner(&client, &firebase_config, &household_id, &actor_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify owner", "details": e})),
            )
                .into_response()
        }
    };

    if !owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only owner can view join requests"})),
        )
            .into_response();
    }

    let url = format!(
        "{}/households/{}/join_requests",
        firebase_config.firestore_url(),
        household_id
    );

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch join requests", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if response.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::OK,
            Json(json!({"success": true, "requests": [], "count": 0})),
        )
            .into_response();
    }

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch join requests", "details": body})),
        )
            .into_response();
    }

    let payload: Value = response.json().await.unwrap_or_else(|_| json!({}));
    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let requests: Vec<Value> = docs
        .iter()
        .map(|doc| {
            json!({
                "request_id": doc.get("name").and_then(|n| n.as_str()).map(extract_document_id).unwrap_or_default(),
                "requester_uid": get_doc_string_field(doc, "requester_uid").unwrap_or_default(),
                "requested_role": get_doc_string_field(doc, "requested_role").unwrap_or_else(|| "member".to_string()),
                "status": get_doc_string_field(doc, "status").unwrap_or_default(),
                "created_at": get_doc_string_field(doc, "created_at")
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "requests": requests,
            "count": requests.len()
        })),
    )
        .into_response()
}

pub async fn review_join_request(
    Path((household_id, request_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<ReviewJoinRequest>,
) -> impl IntoResponse {
    if request.action != "approve" && request.action != "reject" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "action must be 'approve' or 'reject'"})),
        )
            .into_response();
    }

    let client = Client::new();
    let owner = match is_owner(&client, &firebase_config, &household_id, &request.actor_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify owner", "details": e})),
            )
                .into_response()
        }
    };

    if !owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only owner can review join requests"})),
        )
            .into_response();
    }

    let request_doc_url = format!(
        "{}/households/{}/join_requests/{}",
        firebase_config.firestore_url(),
        household_id,
        request_id
    );

    let request_doc_resp = match client.get(&request_doc_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch join request", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !request_doc_resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Join request not found"})),
        )
            .into_response();
    }

    let request_doc: Value = request_doc_resp.json().await.unwrap_or_else(|_| json!({}));
    let requester_uid = get_doc_string_field(&request_doc, "requester_uid").unwrap_or_default();
    let requested_role = get_doc_string_field(&request_doc, "requested_role")
        .unwrap_or_else(|| "member".to_string());

    let patch_request_url = format!(
        "{}/households/{}/join_requests/{}?updateMask.fieldPaths=status&updateMask.fieldPaths=reviewed_at&updateMask.fieldPaths=reviewed_by",
        firebase_config.firestore_url(),
        household_id,
        request_id
    );

    let patch_payload = json!({
        "fields": {
            "status": {"stringValue": if request.action == "approve" { "approved" } else { "rejected" }},
            "reviewed_at": {"stringValue": Utc::now().to_rfc3339()},
            "reviewed_by": {"stringValue": request.actor_uid}
        }
    });

    let review_resp = match client.patch(&patch_request_url).json(&patch_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update join request", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !review_resp.status().is_success() {
        let body = review_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update join request", "details": body})),
        )
            .into_response();
    }

    if request.action == "approve" {
        let member_url = format!(
            "{}/households/{}/members?documentId={}",
            firebase_config.firestore_url(),
            household_id,
            requester_uid
        );

        let member_payload = json!({
            "fields": {
                "uid": {"stringValue": requester_uid},
                "role": {"stringValue": requested_role},
                "status": {"stringValue": "approved"},
                "joined_at": {"stringValue": Utc::now().to_rfc3339()}
            }
        });

        let member_resp = match client.post(&member_url).json(&member_payload).send().await {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Join approved but failed to create member", "details": e.to_string()})),
                )
                    .into_response();
            }
        };

        if !member_resp.status().is_success() {
            let body = member_resp.text().await.unwrap_or_default();
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Join approved but failed to create member", "details": body})),
            )
                .into_response();
        }

        // Store household_id on user doc
        let user_url = format!(
            "{}/users/{}?updateMask.fieldPaths=household_id",
            firebase_config.firestore_url(),
            requester_uid
        );

        let user_payload = json!({
            "fields": {
                "household_id": {"stringValue": household_id}
            }
        });

        let user_resp = client.patch(&user_url).json(&user_payload).send().await;
        if let Err(e) = user_resp {
            eprintln!("Warning: Failed to store household_id on user doc: {}", e);
        }
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "household_id": household_id,
            "request_id": request_id,
            "action": request.action
        })),
    )
        .into_response()
}

pub async fn update_member_role(
    Path((household_id, member_uid)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<UpdateMemberRoleRequest>,
) -> impl IntoResponse {
    let allowed = ["owner", "member"];
    if !allowed.contains(&request.role.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "role must be owner or member"})),
        )
            .into_response();
    }

    let client = Client::new();
    let owner = match is_owner(&client, &firebase_config, &household_id, &request.actor_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify owner", "details": e})),
            )
                .into_response()
        }
    };

    if !owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only owner can update member role"})),
        )
            .into_response();
    }

    let patch_url = format!(
        "{}/households/{}/members/{}?updateMask.fieldPaths=role",
        firebase_config.firestore_url(),
        household_id,
        member_uid
    );

    let payload = json!({
        "fields": {
            "role": {"stringValue": request.role}
        }
    });

    let response = match client.patch(&patch_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update member role", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update member role", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({"success": true, "member_uid": member_uid})),
    )
        .into_response()
}

pub async fn remove_member(
    Path((household_id, member_uid)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<RemoveMemberRequest>,
) -> impl IntoResponse {
    let client = Client::new();
    let owner = match is_owner(&client, &firebase_config, &household_id, &request.actor_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify owner", "details": e})),
            )
                .into_response()
        }
    };

    if !owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only owner can remove members"})),
        )
            .into_response();
    }

    let delete_url = format!(
        "{}/households/{}/members/{}",
        firebase_config.firestore_url(),
        household_id,
        member_uid
    );

    let response = match client.delete(&delete_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to remove member", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to remove member", "details": body})),
        )
            .into_response();
    }

    // Clear household_id from removed member's user doc
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

    let user_resp = client.patch(&user_url).json(&user_payload).send().await;
    if let Err(e) = user_resp {
        eprintln!("Warning: Failed to clear household_id from removed member: {}", e);
    }

    (
        StatusCode::OK,
        Json(json!({"success": true, "removed_uid": member_uid})),
    )
        .into_response()
}

pub async fn list_household_schedules(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let member_uid = query.get("member_uid").cloned().unwrap_or_default();
    if member_uid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "member_uid query is required"})),
        )
            .into_response();
    }

    let client = Client::new();
    let approved = match is_approved_member(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership", "details": e})),
            )
                .into_response()
        }
    };

    if !approved {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only approved members can read schedules"})),
        )
            .into_response();
    }

    let url = format!(
        "{}/households/{}/schedules",
        firebase_config.firestore_url(),
        household_id
    );

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch schedules", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if response.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::OK,
            Json(json!({"success": true, "schedules": [], "count": 0})),
        )
            .into_response();
    }

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch schedules", "details": body})),
        )
            .into_response();
    }

    let payload: Value = response.json().await.unwrap_or_else(|_| json!({}));
    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let schedules: Vec<Value> = docs
        .iter()
        .map(|doc| {
            json!({
                "uid": get_doc_string_field(doc, "uid").unwrap_or_else(|| doc.get("name").and_then(|n| n.as_str()).map(extract_document_id).unwrap_or_default()),
                "name": get_doc_string_field(doc, "name").unwrap_or_default(),
                "time": get_doc_string_field(doc, "time").unwrap_or_default(),
                "date": get_doc_string_field(doc, "date").unwrap_or_default(),
                "expiry": get_doc_string_field(doc, "expiry").unwrap_or_default(),
                "notify": doc.get("fields").and_then(|f| f.get("notify")).and_then(|v| v.get("booleanValue")).and_then(|b| b.as_bool()).unwrap_or(false),
                "category": get_doc_string_field(doc, "category"),
                "quantity": get_doc_string_field(doc, "quantity"),
                "location": get_doc_string_field(doc, "location")
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({"success": true, "schedules": schedules, "count": schedules.len()})),
    )
        .into_response()
}

pub async fn add_household_schedule(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<HouseholdAddScheduleRequest>,
) -> impl IntoResponse {
    let client = Client::new();
    let approved = match is_approved_member(&client, &firebase_config, &household_id, &request.member_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership", "details": e})),
            )
                .into_response()
        }
    };

    if !approved {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only approved members can add schedules"})),
        )
            .into_response();
    }

    let new_id = generate_unique_id("schedule");
    let post_url = format!(
        "{}/households/{}/schedules?documentId={}",
        firebase_config.firestore_url(),
        household_id,
        new_id
    );

    let payload = json!({
        "fields": {
            "uid": {"stringValue": new_id.clone()},
            "name": {"stringValue": request.name},
            "time": {"stringValue": request.time},
            "date": {"stringValue": request.date},
            "expiry": {"stringValue": request.expiry},
            "notify": {"booleanValue": request.notify},
            "category": {"stringValue": request.category.clone().unwrap_or_default()},
            "quantity": {"stringValue": request.quantity},
            "location": {"stringValue": request.location.clone().unwrap_or_default()},
            "created_by": {"stringValue": request.member_uid},
            "created_at": {"stringValue": Utc::now().to_rfc3339()}
        }
    });

    let response = match client.post(&post_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to add schedule", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to add schedule", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({"success": true, "schedule_id": new_id})),
    )
        .into_response()
}

pub async fn list_household_grocery_lists(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let member_uid = query.get("member_uid").cloned().unwrap_or_default();
    if member_uid.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "member_uid query is required"})),
        )
            .into_response();
    }

    let client = Client::new();
    let approved = match is_approved_member(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership", "details": e})),
            )
                .into_response()
        }
    };

    if !approved {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only approved members can read grocery"})),
        )
            .into_response();
    }

    let url = format!(
        "{}/households/{}/grocery",
        firebase_config.firestore_url(),
        household_id
    );

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch grocery", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if response.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::OK,
            Json(json!({"success": true, "lists": [], "count": 0})),
        )
            .into_response();
    }

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch grocery", "details": body})),
        )
            .into_response();
    }

    let payload: Value = response.json().await.unwrap_or_else(|_| json!({}));
    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let lists: Vec<Value> = docs
        .iter()
        .map(|doc| {
            json!({
                "list_id": doc.get("name").and_then(|n| n.as_str()).map(extract_document_id).unwrap_or_default(),
                "list_name": get_doc_string_field(doc, "list_name").unwrap_or_else(|| "Unnamed List".to_string())
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({"success": true, "lists": lists, "count": lists.len()})),
    )
        .into_response()
}

pub async fn create_household_grocery_list(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<HouseholdCreateGroceryListRequest>,
) -> impl IntoResponse {
    let client = Client::new();
    let approved = match is_approved_member(&client, &firebase_config, &household_id, &request.member_uid).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership", "details": e})),
            )
                .into_response()
        }
    };

    if !approved {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only approved members can create grocery lists"})),
        )
            .into_response();
    }

    if request.list_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "list_name is required"})),
        )
            .into_response();
    }

    let list_id = generate_unique_id("grocery");
    let post_url = format!(
        "{}/households/{}/grocery?documentId={}",
        firebase_config.firestore_url(),
        household_id,
        list_id
    );

    let payload = json!({
        "fields": {
            "list_name": {"stringValue": request.list_name},
            "created_by": {"stringValue": request.member_uid},
            "created_at": {"stringValue": Utc::now().to_rfc3339()}
        }
    });

    let response = match client.post(&post_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create grocery list", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create grocery list", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({"success": true, "list_id": list_id, "list_name": request.list_name})),
    )
        .into_response()
}
