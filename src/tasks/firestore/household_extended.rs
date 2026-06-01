// Household schedules and grocery management
// Uses the new host/members[] structure for permission checks

use crate::FirebaseConfig;
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
pub struct HouseholdScheduleRequest {
    pub name: String,
    pub date: String,
    pub time: String,
    #[serde(default)]
    pub expiry: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub quantity: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub notify: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateScheduleRequest {
    pub name: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub expiry: Option<String>,
    pub category: Option<String>,
    pub quantity: Option<String>,
    pub location: Option<String>,
    pub notify: Option<bool>,
}

#[derive(Deserialize)]
pub struct HouseholdGroceryListRequest {
    pub list_name: String,
}

#[derive(Deserialize)]
pub struct AddGroceryItemRequest {
    pub name: String,
    pub quantity: String,
    #[serde(default)]
    pub unit: String,
}

#[derive(Deserialize)]
pub struct UpdateGroceryItemRequest {
    pub name: Option<String>,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub checked: Option<bool>,
}

// Helper: verify member or host access
async fn is_member_or_host(
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

// ============ SCHEDULES ============

pub async fn list_household_schedules(
    Path(household_id): Path<String>,
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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    // Fetch schedules subcollection
    let schedules_url = format!(
        "{}/households/{}/schedules",
        firebase_config.firestore_url(),
        household_id
    );

    let resp = match client.get(&schedules_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch schedules"})),
            )
                .into_response();
        }
    };

    if resp.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "schedules": [],
                "count": 0
            })),
        )
            .into_response();
    }

    if !resp.status().is_success() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch schedules"})),
        )
            .into_response();
    }

    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse schedules"})),
            )
                .into_response();
        }
    };

    let documents = data
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let schedules: Vec<Value> = documents
        .iter()
        .map(|doc| {
            let id = extract_document_id(doc.get("name").and_then(|n| n.as_str()).unwrap_or(""));
            json!({
                "schedule_id": id,
                "name": get_doc_string_field(doc, "name").unwrap_or_default(),
                "date": get_doc_string_field(doc, "date").unwrap_or_default(),
                "time": get_doc_string_field(doc, "time").unwrap_or_default(),
                "created_by": get_doc_string_field(doc, "created_by").unwrap_or_default(),
                "created_at": get_doc_string_field(doc, "created_at").unwrap_or_default()
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "household_id": household_id,
            "schedules": schedules,
            "count": schedules.len()
        })),
    )
        .into_response()
}

pub async fn add_household_schedule(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<HouseholdScheduleRequest>,
) -> impl IntoResponse {
    if request.name.trim().is_empty() || request.date.trim().is_empty() || request.time.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "name, date, and time are required"})),
        )
            .into_response();
    }

    let schedule_id = generate_unique_id("sch");
    let client = Client::new();

    // Create schedule in subcollection
    let schedule_url = format!(
        "{}/households/{}/schedules?documentId={}",
        firebase_config.firestore_url(),
        household_id,
        schedule_id
    );

    let now = Utc::now().to_rfc3339();
    let mut fields = json!({
        "name": {"stringValue": request.name},
        "date": {"stringValue": request.date},
        "time": {"stringValue": request.time},
        "created_at": {"stringValue": now}
    });

    if let Some(expiry) = request.expiry {
        fields["expiry"] = json!({"stringValue": expiry});
    }
    if let Some(category) = request.category {
        fields["category"] = json!({"stringValue": category});
    }
    if let Some(quantity) = request.quantity {
        fields["quantity"] = json!({"stringValue": quantity});
    }
    if let Some(location) = request.location {
        fields["location"] = json!({"stringValue": location});
    }

    let payload = json!({"fields": fields});

    let resp = match client.post(&schedule_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create schedule", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create schedule", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "schedule_id": schedule_id,
            "message": "Schedule created"
        })),
    )
        .into_response()
}

pub async fn update_household_schedule(
    Path((household_id, schedule_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
    Json(request): Json<UpdateScheduleRequest>,
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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let schedule_url = format!(
        "{}/households/{}/schedules/{}",
        firebase_config.firestore_url(),
        household_id,
        schedule_id
    );

    let mut update_paths = Vec::new();
    let mut fields = serde_json::Map::new();

    if let Some(name) = request.name {
        fields.insert("name".to_string(), json!({"stringValue": name}));
        update_paths.push("name");
    }
    if let Some(date) = request.date {
        fields.insert("date".to_string(), json!({"stringValue": date}));
        update_paths.push("date");
    }
    if let Some(time) = request.time {
        fields.insert("time".to_string(), json!({"stringValue": time}));
        update_paths.push("time");
    }
    if let Some(expiry) = request.expiry {
        fields.insert("expiry".to_string(), json!({"stringValue": expiry}));
        update_paths.push("expiry");
    }
    if let Some(category) = request.category {
        fields.insert("category".to_string(), json!({"stringValue": category}));
        update_paths.push("category");
    }
    if let Some(quantity) = request.quantity {
        fields.insert("quantity".to_string(), json!({"stringValue": quantity}));
        update_paths.push("quantity");
    }
    if let Some(location) = request.location {
        fields.insert("location".to_string(), json!({"stringValue": location}));
        update_paths.push("location");
    }

    if update_paths.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No fields to update"})),
        )
            .into_response();
    }

    let field_paths = update_paths.iter()
        .map(|p| format!("&updateMask.fieldPaths={}", p))
        .collect::<Vec<_>>()
        .join("");

    let update_url = format!("{}?updateMask.fieldPaths={}{}", schedule_url, update_paths[0], field_paths);

    let payload = json!({"fields": Value::Object(fields)});

    let resp = match client.patch(&update_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update schedule", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update schedule", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Schedule updated"
        })),
    )
        .into_response()
}

pub async fn delete_household_schedule(
    Path((household_id, schedule_id)): Path<(String, String)>,
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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let schedule_url = format!(
        "{}/households/{}/schedules/{}",
        firebase_config.firestore_url(),
        household_id,
        schedule_id
    );

    let resp = match client.delete(&schedule_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete schedule", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Schedule not found"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Schedule deleted"
        })),
    )
        .into_response()
}

// ============ GROCERY LISTS ============

pub async fn list_household_grocery_lists(
    Path(household_id): Path<String>,
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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let grocery_url = format!(
        "{}/households/{}/grocery",
        firebase_config.firestore_url(),
        household_id
    );

    let resp = match client.get(&grocery_url).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch grocery lists"})),
            )
                .into_response();
        }
    };

    if resp.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "lists": [],
                "count": 0
            })),
        )
            .into_response();
    }

    if !resp.status().is_success() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch grocery lists"})),
        )
            .into_response();
    }

    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse grocery lists"})),
            )
                .into_response();
        }
    };

    let documents = data
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let lists: Vec<Value> = documents
        .iter()
        .map(|doc| {
            let id = extract_document_id(doc.get("name").and_then(|n| n.as_str()).unwrap_or(""));
            json!({
                "list_id": id,
                "name": get_doc_string_field(doc, "name").unwrap_or_default(),
                "created_by": get_doc_string_field(doc, "created_by").unwrap_or_default(),
                "created_at": get_doc_string_field(doc, "created_at").unwrap_or_default()
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "household_id": household_id,
            "lists": lists,
            "count": lists.len()
        })),
    )
        .into_response()
}

pub async fn create_household_grocery_list(
    Path(household_id): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
    Json(request): Json<HouseholdGroceryListRequest>,
) -> impl IntoResponse {
    if request.list_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "list_name is required"})),
        )
            .into_response();
    }

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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let list_id = generate_unique_id("gro");
    let now = Utc::now().to_rfc3339();

    let grocery_url = format!(
        "{}/households/{}/grocery?documentId={}",
        firebase_config.firestore_url(),
        household_id,
        list_id
    );

    let payload = json!({
        "fields": {
            "name": {"stringValue": request.list_name},
            "created_by": {"stringValue": member_uid},
            "created_at": {"stringValue": now}
        }
    });

    let resp = match client.post(&grocery_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create list", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create list", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "list_id": list_id,
            "message": "Grocery list created"
        })),
    )
        .into_response()
}

pub async fn delete_household_grocery_list(
    Path((household_id, list_id)): Path<(String, String)>,
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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let grocery_url = format!(
        "{}/households/{}/grocery/{}",
        firebase_config.firestore_url(),
        household_id,
        list_id
    );

    let resp = match client.delete(&grocery_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete list", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "List not found"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "List deleted"
        })),
    )
        .into_response()
}

pub async fn add_household_grocery_item(
    Path((household_id, list_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
    Json(request): Json<AddGroceryItemRequest>,
) -> impl IntoResponse {
    if request.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "name is required"})),
        )
            .into_response();
    }

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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let item_id = generate_unique_id("item");

    let item_url = format!(
        "{}/households/{}/grocery/{}/items?documentId={}",
        firebase_config.firestore_url(),
        household_id,
        list_id,
        item_id
    );

    let payload = json!({
        "fields": {
            "name": {"stringValue": request.name},
            "quantity": {"stringValue": request.quantity},
            "unit": {"stringValue": request.unit},
            "checked": {"booleanValue": false}
        }
    });

    let resp = match client.post(&item_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to add item", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to add item", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "item_id": item_id,
            "message": "Item added"
        })),
    )
        .into_response()
}

pub async fn update_household_grocery_item(
    Path((household_id, list_id, item_id)): Path<(String, String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Query(query): Query<HashMap<String, String>>,
    Json(request): Json<UpdateGroceryItemRequest>,
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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let item_url = format!(
        "{}/households/{}/grocery/{}/items/{}",
        firebase_config.firestore_url(),
        household_id,
        list_id,
        item_id
    );

    let mut update_paths = Vec::new();
    let mut fields = serde_json::Map::new();

    if let Some(name) = request.name {
        fields.insert("name".to_string(), json!({"stringValue": name}));
        update_paths.push("name");
    }
    if let Some(quantity) = request.quantity {
        fields.insert("quantity".to_string(), json!({"stringValue": quantity}));
        update_paths.push("quantity");
    }
    if let Some(unit) = request.unit {
        fields.insert("unit".to_string(), json!({"stringValue": unit}));
        update_paths.push("unit");
    }
    if let Some(checked) = request.checked {
        fields.insert("checked".to_string(), json!({"booleanValue": checked}));
        update_paths.push("checked");
    }

    if update_paths.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No fields to update"})),
        )
            .into_response();
    }

    let field_paths = update_paths.iter()
        .map(|p| format!("&updateMask.fieldPaths={}", p))
        .collect::<Vec<_>>()
        .join("");

    let update_url = format!("{}?updateMask.fieldPaths={}{}", item_url, update_paths[0], field_paths);

    let payload = json!({"fields": Value::Object(fields)});

    let resp = match client.patch(&update_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update item", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update item", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Item updated"
        })),
    )
        .into_response()
}

pub async fn delete_household_grocery_item(
    Path((household_id, list_id, item_id)): Path<(String, String, String)>,
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

    // Verify membership
    match is_member_or_host(&client, &firebase_config, &household_id, &member_uid).await {
        Ok(true) => {},
        Ok(false) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "Not a member of this household"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify membership"})),
            )
                .into_response();
        }
    }

    let item_url = format!(
        "{}/households/{}/grocery/{}/items/{}",
        firebase_config.firestore_url(),
        household_id,
        list_id,
        item_id
    );

    let resp = match client.delete(&item_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete item", "details": e.to_string()})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Item not found"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Item deleted"
        })),
    )
        .into_response()
}
