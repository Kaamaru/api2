use crate::{
    structures::structs::{
        AddGroceryItemRequest, CreateGroceryListRequest, GroceryItem, GroceryList,
        GroceryItemField, GroceryListField, UpdateGroceryItemRequest,
    },
    FirebaseConfig,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Router,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

fn generate_unique_id() -> String {
    let now = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let uuid = Uuid::new_v4().simple();
    format!("{}_{}", uuid, now)
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

fn get_doc_bool_field(doc: &Value, key: &str) -> Option<bool> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("booleanValue"))
        .and_then(|b| b.as_bool())
}

fn parse_grocery_items_from_documents(documents: &[Value]) -> Vec<GroceryItem> {
    let mut items = Vec::new();

    for doc in documents {
        let uid = doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(extract_document_id)
            .unwrap_or_default();

        items.push(GroceryItem {
            uid,
            name: get_doc_string_field(doc, "name").unwrap_or_default(),
            quantity: get_doc_string_field(doc, "quantity").unwrap_or_default(),
            unit: get_doc_string_field(doc, "unit").unwrap_or_default(),
            checked: get_doc_bool_field(doc, "checked").unwrap_or(false),
        });
    }

    items
}

async fn fetch_grocery_items_for_list(
    client: &Client,
    firebase_config: &FirebaseConfig,
    userid: &str,
    list_id: &str,
) -> Result<Vec<GroceryItem>, String> {
    let items_url = format!(
        "{}/users/{}/grocery/{}/items",
        firebase_config.firestore_url(),
        userid,
        list_id
    );

    let resp = client
        .get(&items_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch grocery items: {e}"))?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch items {status}: {body}"));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse grocery items: {e}"))?;

    let docs: Vec<Value> = data
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(parse_grocery_items_from_documents(&docs))
}

// ===== GET Endpoints =====

/// Get all grocery lists for a user
pub async fn get_grocery_lists(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!("get_grocery_lists for user: {}", userid);

    let client = Client::new();
    let url = format!("{}/users/{}/grocery", firebase_config.firestore_url(), userid);

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to fetch grocery lists: {}", e);
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
        let body = resp.text().await.unwrap_or_default();
        eprintln!("❌ Failed to fetch grocery lists: {}", body);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch grocery lists", "details": body})),
        )
            .into_response();
    }

    let data: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Failed to parse grocery list response: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse grocery lists"})),
            )
                .into_response();
        }
    };

    let documents: Vec<Value> = data
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lists = Vec::new();
    for doc in &documents {
        let list_id = doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(extract_document_id)
            .unwrap_or_default();

        if list_id.is_empty() {
            continue;
        }

        let list_name = get_doc_string_field(doc, "list_name")
            .unwrap_or_else(|| "Unnamed List".to_string());

        let items = match fetch_grocery_items_for_list(&client, &firebase_config, &userid, &list_id).await {
            Ok(items) => items,
            Err(e) => {
                eprintln!("❌ Failed to fetch items for list {}: {}", list_id, e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Failed to fetch grocery items", "details": e})),
                )
                    .into_response();
            }
        };

        lists.push(GroceryList {
            list_id,
            list_name,
            items,
        });
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "lists": lists,
            "count": lists.len()
        })),
    )
        .into_response()
}

/// Get a single grocery list
pub async fn get_grocery_list(
    Path((userid, list_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!("get_grocery_list: user={}, list_id={}", userid, list_id);

    let client = Client::new();
    let list_url = format!(
        "{}/users/{}/grocery/{}",
        firebase_config.firestore_url(),
        userid,
        list_id
    );

    let list_resp = match client.get(&list_url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to fetch grocery list: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch grocery list"})),
            )
                .into_response();
        }
    };

    if list_resp.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Grocery list not found"})),
        )
            .into_response();
    }

    if !list_resp.status().is_success() {
        let body = list_resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch grocery list", "details": body})),
        )
            .into_response();
    }

    let list_doc: Value = match list_resp.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Failed to parse grocery list: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse grocery list"})),
            )
                .into_response();
        }
    };

    let items = match fetch_grocery_items_for_list(&client, &firebase_config, &userid, &list_id).await {
        Ok(items) => items,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch grocery items", "details": e})),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "list": {
                "list_id": list_id,
                "list_name": get_doc_string_field(&list_doc, "list_name").unwrap_or_else(|| "Unnamed List".to_string()),
                "items": items,
            }
        })),
    )
        .into_response()
}

// ===== CREATE Endpoints =====

/// Create a new grocery list
pub async fn create_grocery_list(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<CreateGroceryListRequest>,
) -> impl IntoResponse {
    println!("create_grocery_list: user={}", userid);

    let client = Client::new();
    let list_id = format!("grocery_{}", generate_unique_id());

    let post_url = format!(
        "{}/users/{}/grocery?documentId={}",
        firebase_config.firestore_url(),
        userid,
        list_id
    );

    let firestore_payload = json!({
        "fields": {
            "list_name": { "stringValue": request.list_name }
        }
    });

    let patch_response = match client.post(&post_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch grocery list: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create grocery list"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create grocery list", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": format!("Grocery list '{}' created", request.list_name),
            "list": {
                "list_id": list_id,
                "list_name": request.list_name,
                "items": []
            }
        })),
    )
        .into_response()
}

// ===== ADD Item Endpoint =====

/// Add item to grocery list
pub async fn add_grocery_item(
    Path((userid, list_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<AddGroceryItemRequest>,
) -> impl IntoResponse {
    println!("add_grocery_item: user={}, list_id={}", userid, list_id);

    let client = Client::new();
    let item_id = format!("item_{}", generate_unique_id());

    let post_url = format!(
        "{}/users/{}/grocery/{}/items?documentId={}",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id
    );

    let firestore_payload = json!({
        "fields": {
            "uid": { "stringValue": item_id.clone() },
            "name": { "stringValue": request.name },
            "quantity": { "stringValue": request.quantity },
            "unit": { "stringValue": request.unit },
            "checked": { "booleanValue": false }
        }
    });

    let patch_response = match client.post(&post_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch grocery item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to add grocery item"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to add grocery item", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": format!("Item '{}' added to list", request.name),
            "item": {
                "uid": item_id,
                "name": request.name,
                "quantity": request.quantity,
                "unit": request.unit,
                "checked": false
            }
        })),
    )
        .into_response()
}

// ===== UPDATE Item Endpoint =====

/// Update grocery item (toggle checked, update quantity)
pub async fn update_grocery_item(
    Path((userid, list_id, item_id)): Path<(String, String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<UpdateGroceryItemRequest>,
) -> impl IntoResponse {
    println!(
        "update_grocery_item: user={}, list_id={}, item_id={}",
        userid, list_id, item_id
    );

    let client = Client::new();
    let mut field_paths = Vec::new();
    let mut item_fields = serde_json::Map::new();

    // Build item fields based on what was provided
    if let Some(checked) = request.checked {
        field_paths.push("checked".to_string());
        item_fields.insert("checked".to_string(), json!({"booleanValue": checked}));
    }

    if let Some(quantity) = &request.quantity {
        field_paths.push("quantity".to_string());
        item_fields.insert("quantity".to_string(), json!({"stringValue": quantity}));
    }

    if let Some(unit) = &request.unit {
        field_paths.push("unit".to_string());
        item_fields.insert("unit".to_string(), json!({"stringValue": unit}));
    }

    if field_paths.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No fields to update"})),
        )
            .into_response();
    }

    let patch_url = format!(
        "{}/users/{}/grocery/{}/items/{}?updateMask.fieldPaths={}",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id,
        field_paths.join("&updateMask.fieldPaths=")
    );

    let firestore_payload = json!({
        "fields": serde_json::Value::Object(item_fields)
    });

    eprintln!("📤 UPDATE payload: {}", firestore_payload);

    let patch_response = match client.patch(&patch_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch grocery item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update grocery item"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update grocery item"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Grocery item updated",
            "updated_fields": serde_json::to_value(&request).unwrap_or(json!({}))
        })),
    )
        .into_response()
}

// ===== DELETE Endpoints =====

/// Delete entire grocery list
pub async fn delete_grocery_list(
    Path((userid, list_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!("delete_grocery_list: user={}, list_id={}", userid, list_id);

    let client = Client::new();
    let delete_url = format!(
        "{}/users/{}/grocery/{}",
        firebase_config.firestore_url(),
        userid,
        list_id
    );

    let patch_response = match client.delete(&delete_url).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to delete list: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete list"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Delete list failed: {}", body);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete list"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("Grocery list '{}' deleted", list_id)
        })),
    )
        .into_response()
}

/// Delete single grocery item
pub async fn delete_grocery_item(
    Path((userid, list_id, item_id)): Path<(String, String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!(
        "delete_grocery_item: user={}, list_id={}, item_id={}",
        userid, list_id, item_id
    );

    let client = Client::new();

    let delete_url = format!(
        "{}/users/{}/grocery/{}/items/{}",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id
    );

    let patch_response = match client.delete(&delete_url).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to delete item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete item"})),
            )
                .into_response();
        }
    };

    if patch_response.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Item not found", "success": false})),
        )
            .into_response();
    }

    if !patch_response.status().is_success() {
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Delete item failed: {}", body);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete item"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("Grocery item '{}' deleted", item_id)
        })),
    )
        .into_response()
}

// ===== Helper Functions =====

// ===== CONVERSION Endpoint =====

/// Convert checked grocery item to schedule
pub async fn convert_grocery_to_schedule(
    Path((userid, list_id, item_id)): Path<(String, String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<crate::structures::structs::GroceryToScheduleRequest>,
) -> impl IntoResponse {
    println!(
        "convert_grocery_to_schedule: user={}, list_id={}, item_id={}",
        userid, list_id, item_id
    );

    let client = Client::new();

    // STEP 1: Fetch the grocery item details from subcollection doc
    let get_url = format!(
        "{}/users/{}/grocery/{}/items/{}",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id
    );
    let resp = match client.get(&get_url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to fetch user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch grocery item"})),
            )
                .into_response();
        }
    };

    if resp.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Grocery item not found"})),
        )
            .into_response();
    }

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch grocery item", "details": body})),
        )
            .into_response();
    }

    let item_doc: Value = match resp.json().await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("❌ Failed to parse grocery item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse grocery item"})),
            )
                .into_response();
        }
    };

    let item_name = get_doc_string_field(&item_doc, "name")
        .unwrap_or("Unknown".to_string())
        .to_string();

    let item_quantity = get_doc_string_field(&item_doc, "quantity")
        .unwrap_or("1".to_string())
        .to_string();

    let item_unit = get_doc_string_field(&item_doc, "unit")
        .unwrap_or("".to_string())
        .to_string();

    // STEP 2: Create a schedule document in subcollection
    let schedule_id = format!("schedule_{}", generate_unique_id());
    let schedule_post_url = format!(
        "{}/users/{}/schedules?documentId={}",
        firebase_config.firestore_url(),
        userid,
        schedule_id
    );

    let quantity_str = if item_unit.is_empty() {
        item_quantity.clone()
    } else {
        format!("{} {}", item_quantity, item_unit)
    };

    let schedule_payload = json!({
        "fields": {
            "uid": { "stringValue": schedule_id.clone() },
            "name": { "stringValue": item_name },
            "quantity": { "stringValue": quantity_str },
            "date": { "stringValue": request.date },
            "time": { "stringValue": request.time },
            "expiry": { "stringValue": request.expiry },
            "category": { "stringValue": request.category.clone().unwrap_or_else(|| "grocery".to_string()) },
            "location": { "stringValue": request.location.clone().unwrap_or_default() },
            "notify": { "booleanValue": false }
        }
    });

    let schedule_response = match client
        .post(&schedule_post_url)
        .json(&schedule_payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to create schedule: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create schedule"})),
            )
                .into_response();
        }
    };

    if !schedule_response.status().is_success() {
        let status = schedule_response.status();
        let body = schedule_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Schedule creation failed: {}\n{}", status, body);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create schedule", "details": body})),
        )
            .into_response();
    }

    // STEP 3: Mark grocery item as checked via item doc patch
    let grocery_patch_url = format!(
        "{}/users/{}/grocery/{}/items/{}?updateMask.fieldPaths=checked",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id
    );

    let grocery_payload = json!({
        "fields": {
            "checked": { "booleanValue": true }
        }
    });

    let grocery_response = match client
        .patch(&grocery_patch_url)
        .json(&grocery_payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to update grocery item: {}", e);
            // Schedule was created but grocery item marking failed - not ideal but partial success
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Schedule created but failed to mark item as checked",
                    "schedule_id": schedule_id
                })),
            )
                .into_response();
        }
    };

    if !grocery_response.status().is_success() {
        let status = grocery_response.status();
        let body = grocery_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Grocery item update failed: {}\\n{}", status, body);
        // Again, schedule created but grocery marker failed
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Schedule created but failed to mark item as checked",
                "schedule_id": schedule_id,
                "details": body
            })),
        )
            .into_response();
    }

    // SUCCESS: Both operations completed
    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Grocery item converted to schedule",
            "schedule": {
                "schedule_id": schedule_id,
                "name": item_name,
                "quantity": quantity_str,
                "date": request.date,
                "time": request.time,
                "expiry": request.expiry,
                "category": request.category.unwrap_or_else(|| "grocery".to_string()),
                "location": request.location.unwrap_or_default()
            },
            "grocery_item": {
                "item_id": item_id,
                "checked": true
            }
        })),
    )
        .into_response()
}
